//! SolidCraft on Android.
//!
//! Runs the same [`solidcraft_ui_egui::SolveApp`] as the desktop app through eframe on a
//! `NativeActivity` (wgpu: Vulkan where the device has it, GLES otherwise). Build the APK from
//! `android/` (see `android/README.md`); `cargo ndk` builds this crate into its `jniLibs`.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

// The desktop app's loopback control server, unchanged: the MCP server's sessions reach the app
// through it, as `solidcraft-cli mcp` reaches the desktop app.
#[cfg(target_os = "android")]
#[allow(dead_code)]
#[path = "../../solidcraft/src/control_server.rs"]
mod control_server;
#[cfg(target_os = "android")]
mod documents;
#[cfg(target_os = "android")]
mod mcp_host;
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
mod mcp_http;
// Plain functions, built (and tested) on every system.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
mod names;

#[cfg(target_os = "android")]
mod android {
    use std::cell::RefCell;
    use std::rc::Rc;

    use solidcraft_engine::Session;
    use solidcraft_ui_egui::{Services, SolveApp};
    use winit::platform::android::activity::AndroidApp;

    use crate::documents::Documents;
    use crate::mcp_host::McpHost;

    /// The viewport scene's depth and stencil buffers and anti-aliasing, as on the desktop: a
    /// 24-bit depth buffer is the one every Android GPU has. The scene draws offscreen with them;
    /// the window's pass (egui's) has no depth and one sample.
    const DEPTH_BITS: u8 = 24;
    const STENCIL_BITS: u8 = 8;
    const MSAA: u16 = 4;

    struct Shell {
        app: SolveApp,
        /// Frames drawn, up to two: the second means the first was presented.
        frames: u32,
        /// The document pickers and the local copies of documents.
        documents: Option<Rc<RefCell<Documents>>>,
        /// Back closes what is open: Escape goes into the next frame's input.
        escape: bool,
        /// Settings › MCP server.
        mcp: McpHost,
    }

    /// Whether Escape has something to close: Preferences, a dialog, a tool, a sketch being
    /// edited, a menu or popup, or a field with the keyboard.
    fn escape_closes(app: &SolveApp, ctx: &egui::Context) -> bool {
        app.prefs_window.open
            || app.dialog.is_some()
            || app.tool.is_some()
            || app.session.active_sketch.is_some()
            || app.ui.palette_open
            || app.viewport.context_menu.is_some()
            || egui::Popup::is_any_open(ctx)
            || ctx.memory(|m| m.focused().is_some())
    }

    impl eframe::App for Shell {
        fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            if self.frames < 2 {
                self.frames += 1;
                ctx.request_repaint();
                if self.frames == 2 {
                    solidcraft_engine::crash::phase(solidcraft_engine::crash::RUNNING);
                }
            }
            // The design on screen is the one whose commands a crash report replays.
            self.app.session.crash_journal = solidcraft_engine::crash::enabled();
            // Before the app's logic, which takes a picker's answer this frame.
            if let Some(d) = &self.documents {
                d.borrow_mut().tick(&mut self.app, ctx);
                // Back: close what is open, as Escape; with nothing open, leave as Home does.
                if d.borrow_mut().take_back() {
                    if escape_closes(&self.app, ctx) {
                        self.escape = true;
                        ctx.request_repaint();
                    } else {
                        d.borrow().to_background();
                    }
                }
            }
            self.mcp.tick(&mut self.app);
            self.app.logic(ctx);
        }
        fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
            if std::mem::take(&mut self.escape) {
                for pressed in [true, false] {
                    raw.events.push(egui::Event::Key {
                        key: egui::Key::Escape,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
            }
            self.app.raw_input_hook(raw);
        }
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            self.app.ui(ui);
            // egui's Copy has no clipboard on Android: hand what it copied to the system's.
            if let Some(d) = &self.documents {
                let copied: Vec<String> = ui.ctx().output(|o| {
                    o.commands
                        .iter()
                        .filter_map(|c| match c {
                            egui::OutputCommand::CopyText(text) => Some(text.clone()),
                            _ => None,
                        })
                        .collect()
                });
                for text in copied {
                    d.borrow().copy_text(&text);
                }
            }
        }
        fn save(&mut self, storage: &mut dyn eframe::Storage) {
            storage.set_string(solidcraft_ui_egui::prefs::STORAGE_KEY, self.app.prefs());
            if let Some(mcp) = McpHost::stored(&self.app) {
                storage.set_string(crate::mcp_host::STORAGE_KEY, mcp);
            }
        }
        fn on_exit(&mut self) {
            // A save still being written ends before the app does.
            solidcraft_ui_egui::save_job::tick(&mut self.app, true);
            solidcraft_ui_egui::recovery_ui::close(&mut self.app);
            if let Some(d) = &self.documents {
                d.borrow_mut().flush();
            }
            solidcraft_engine::crash::end_session();
        }
    }

    /// egui-wgpu is about to start a step of getting the graphics device and the surface up: the
    /// crash journal keeps it, and the adapter once it is picked.
    fn startup_step(step: &'static str, detail: &str) {
        if step == eframe::egui_wgpu::STEP_ADAPTER_CHOSEN {
            solidcraft_engine::crash::set_environment(&format!("Renderer: {detail}"));
        }
        solidcraft_engine::crash::phase(step);
    }

    /// Open, save and attach through the system's document pickers.
    fn services(documents: Option<&Rc<RefCell<Documents>>>) -> Services {
        let Some(documents) = documents else { return Services::default() };
        let (open, save, file) = (documents.clone(), documents.clone(), documents.clone());
        Services {
            pick_open: Some(Box::new(move |_: &[&str]| open.borrow_mut().pick_open())),
            pick_save: Some(Box::new(move |name: &str, _: &[&str]| save.borrow_mut().pick_save(name))),
            pick_file: Some(Box::new(move |_: &str, _: &[&str]| file.borrow_mut().pick_open())),
            ..Default::default()
        }
    }

    /// The viewport scene's sample count: `samples` clamped by the backend's policy (GLES renders
    /// 1x), or the largest the device has up to the default when it can't do that many. As the
    /// desktop app's `scene_samples`.
    fn scene_samples(backend: eframe::wgpu::Backend, samples: u32, supported: &[u32]) -> u32 {
        let requested = eframe::egui_wgpu::effective_msaa_samples(backend, samples.max(1));
        if requested <= 1 || supported.is_empty() || supported.contains(&requested) {
            requested
        } else {
            supported.iter().copied().filter(|&n| n <= solidcraft_ui_egui::prefs::Prefs::default().msaa).max().unwrap_or(1)
        }
    }

    pub fn run(app: AndroidApp) {
        android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("SolidCraft"));
        // The app's own folder: crash reports and eframe's settings live there.
        let data = app.internal_data_path();
        // The MCP agent's files: where a file manager or USB can reach them, when there is such a place.
        let agent_folder = app.external_data_path().or_else(|| data.clone()).map(|d| d.join("Agent")).unwrap_or_default();
        let activity = app.activity_as_ptr();
        if let Some(dir) = data.as_ref().map(|d| d.join("crashes")) {
            let build = solidcraft_engine::crash::Build {
                version: solidcraft_ui_egui::help::VERSION,
                commit: solidcraft_ui_egui::help::COMMIT,
                built: solidcraft_ui_egui::help::BUILD_DATE,
            };
            solidcraft_engine::crash::install(dir, build);
        }
        eframe::egui_wgpu::set_startup_hook(startup_step);
        solidcraft_engine::crash::phase("window");
        let options = eframe::NativeOptions {
            android_app: Some(app),
            depth_buffer: 0,
            stencil_buffer: 0,
            multisampling: 1,
            persistence_path: data.as_ref().map(|d| d.join("app.ron")),
            ..Default::default()
        };
        let result = eframe::run_native(
            "SolidCraft",
            options,
            Box::new(|cc| {
                solidcraft_engine::crash::phase("app setup");
                // The bundled faces at once; the system's CJK, Hangul and Thai fallbacks are found on
                // a thread, so the first frame doesn't wait for them.
                solidcraft_ui_egui::fonts::setup_bundled_fonts(&cc.egui_ctx);
                solidcraft_ui_egui::fonts::load_system_fonts_in_background(&cc.egui_ctx);
                // No updates or agent services: the host has none to offer.
                let documents = data.as_deref().and_then(|d| Documents::new(d, activity)).map(|d| Rc::new(RefCell::new(d)));
                // Requests from the MCP server's sessions, through the loopback control channel.
                let (control, requests) = std::sync::mpsc::channel();
                let mut app = SolveApp::new(Session::default(), services(documents.as_ref())).with_control(requests);
                let stored_mcp = cc.storage.and_then(|s| s.get_string(crate::mcp_host::STORAGE_KEY));
                app.mcp_server = Some(Box::new(McpHost::page(stored_mcp)));
                let checks = documents.clone();
                let on_screen = Box::new(move || -> crate::mcp_http::Ready {
                    match &checks {
                        Some(d) => std::sync::Arc::new(d.borrow().on_screen()),
                        None => std::sync::Arc::new(|| true),
                    }
                });
                let mcp = McpHost::new(control, cc.egui_ctx.clone(), agent_folder, on_screen);
                // OK evaluates off the window thread, as on the desktop: a feature that takes
                // longer than Android's input timeout (a gear, a helical thread, a large boolean)
                // would otherwise have the system offer to close the app as not responding.
                app.apply_off_thread = Some(solidcraft_ui_egui::dialogs::APPLY_AT_ONCE);
                if let Some(p) = cc.storage.and_then(|s| solidcraft_ui_egui::prefs::stored(|k| s.get_string(k))) {
                    app.load_prefs(&p);
                }
                // An agent's commands (the MCP server) compute off the window thread, always:
                // Android closes an app whose window thread answers no input for 10 seconds, and a
                // boolean on a gear can take minutes on a tablet. The window goes on drawing,
                // shows the command computing with a Stop button, and answers `pending` and
                // inspection meanwhile.
                app.preferences.agent_off_thread = true;
                if let Some(rs) = &cc.wgpu_render_state {
                    let info = rs.adapter.get_info();
                    let samples = scene_samples(info.backend, u32::from(MSAA), &rs.msaa_supported);
                    log::info!("graphics: {:?} {} ({}), MSAA {samples}x", info.backend, info.name, info.driver);
                    solidcraft_engine::crash::phase("viewport pipelines (shader compile)");
                    app.set_wgpu_split(rs, DEPTH_BITS, STENCIL_BITS, samples);
                    solidcraft_ui_egui::gpu::install_picture_renderer(rs);
                    solidcraft_ui_egui::prefs::check_msaa(&mut app, u32::from(MSAA));
                    solidcraft_engine::crash::set_environment(&format!(
                        "Renderer: {:?} {} ({}), MSAA {samples}x",
                        info.backend, info.name, info.driver
                    ));
                }
                Ok(Box::new(Shell { app, frames: 0, documents, escape: false, mcp }))
            }),
        );
        if let Err(e) = result {
            log::error!("SolidCraft could not start: {e}");
        }
        // The system destroyed the activity, and the app has shut down (designs saved, document
        // copies flushed). winit's event loop can only be made once per process, so a new
        // activity in this process could never start: end the process, and the next launch
        // starts a fresh one.
        log::info!("the activity ended; exiting");
        std::process::exit(0);
    }
}

/// The activity's entry point: android-activity's glue calls it by this name on its own thread.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android::run(app);
}
