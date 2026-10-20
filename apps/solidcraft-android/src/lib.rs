//! SolidCraft on Android.
//!
//! Runs the same [`solidcraft_ui_egui::SolveApp`] as the desktop app through eframe on a
//! `NativeActivity` (wgpu: Vulkan where the device has it, GLES otherwise). Build the APK from
//! `android/` (see `android/README.md`); `cargo ndk` builds this crate into its `jniLibs`.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_os = "android")]
mod documents;
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

    /// The viewport's depth and stencil buffers, as on the desktop: a 24-bit depth buffer is the
    /// one every Android GPU has.
    const DEPTH_BITS: u8 = 24;
    const STENCIL_BITS: u8 = 8;
    const MSAA: u16 = 4;

    struct Shell {
        app: SolveApp,
        /// Frames drawn, up to two: the second means the first was presented.
        frames: u32,
        /// The document pickers and the local copies of documents.
        documents: Option<Rc<RefCell<Documents>>>,
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
            }
            self.app.logic(ctx);
        }
        fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
            self.app.raw_input_hook(raw);
        }
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            self.app.ui(ui);
        }
        fn save(&mut self, storage: &mut dyn eframe::Storage) {
            storage.set_string(solidcraft_ui_egui::prefs::STORAGE_KEY, self.app.prefs());
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

    pub fn run(app: AndroidApp) {
        android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("SolidCraft"));
        // The app's own folder: crash reports and eframe's settings live there.
        let data = app.internal_data_path();
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
            depth_buffer: DEPTH_BITS,
            stencil_buffer: STENCIL_BITS,
            multisampling: MSAA,
            persistence_path: data.as_ref().map(|d| d.join("app.ron")),
            ..Default::default()
        };
        let result = eframe::run_native(
            "SolidCraft",
            options,
            Box::new(|cc| {
                solidcraft_engine::crash::phase("app setup");
                solidcraft_ui_egui::fonts::setup_fonts(&cc.egui_ctx);
                // No updates or agent services: the host has none to offer.
                let documents = data.as_deref().and_then(|d| Documents::new(d, activity)).map(|d| Rc::new(RefCell::new(d)));
                let mut app = SolveApp::new(Session::default(), services(documents.as_ref()));
                if let Some(p) = cc.storage.and_then(|s| solidcraft_ui_egui::prefs::stored(|k| s.get_string(k))) {
                    app.load_prefs(&p);
                }
                if let Some(rs) = &cc.wgpu_render_state {
                    let info = rs.adapter.get_info();
                    log::info!("graphics: {:?} {} ({}), MSAA {}x", info.backend, info.name, info.driver, rs.msaa_samples);
                    solidcraft_engine::crash::phase("viewport pipelines (shader compile)");
                    app.set_wgpu(rs, DEPTH_BITS, STENCIL_BITS, rs.msaa_samples);
                    solidcraft_ui_egui::gpu::install_picture_renderer(rs);
                    solidcraft_ui_egui::prefs::check_msaa(&mut app, u32::from(MSAA));
                    solidcraft_engine::crash::set_environment(&format!(
                        "Renderer: {:?} {} ({}), MSAA {}x",
                        info.backend, info.name, info.driver, rs.msaa_samples
                    ));
                }
                Ok(Box::new(Shell { app, frames: 0, documents }))
            }),
        );
        if let Err(e) = result {
            log::error!("SolidCraft could not start: {e}");
        }
    }
}

/// The activity's entry point: android-activity's glue calls it by this name on its own thread.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android::run(app);
}
