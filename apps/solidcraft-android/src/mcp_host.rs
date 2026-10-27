//! Settings › MCP server: runs the endpoint the page asks for (`crate::mcp_http`).
//!
//! The endpoint's sessions are bridged, as `solidcraft-cli mcp` is, to the app's loopback control
//! channel (the desktop app's own control server, started on first use with a fresh token that
//! stays in the app's private folder). The page's settings are kept across launches.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use solidcraft_ui_egui::i18n::{format, tr};
use solidcraft_ui_egui::prefs::mcp_server::{McpServerPage, McpServerSettings};
use solidcraft_ui_egui::{ControlRequest, SolveApp};

use crate::mcp_http::{Endpoint, MakeServer, Ready, lan_address, new_token};

/// Where the settings are kept in eframe's storage.
pub const STORAGE_KEY: &str = "android_mcp_server";
/// How often the device's network address is looked up again (Wi-Fi comes and goes).
const LAN_EVERY: Duration = Duration::from_secs(5);

/// What the endpoint was started with; a change restarts it.
#[derive(Clone, PartialEq, Eq)]
struct Wanted {
    network: bool,
    port: u16,
    token: String,
}

pub struct McpHost {
    control: Sender<ControlRequest>,
    ctx: egui::Context,
    /// The loopback control channel's port, once started.
    control_port: Option<u16>,
    endpoint: Option<(Endpoint, Wanted)>,
    /// Why the endpoint isn't running, until the settings change.
    failed: Option<(Wanted, String)>,
    /// The folder the agent may use.
    folder: PathBuf,
    lan: Option<String>,
    lan_at: Option<Instant>,
    /// Makes the check of whether the app is on screen, for each endpoint started.
    on_screen: Box<dyn Fn() -> Ready>,
}

impl McpHost {
    pub fn new(control: Sender<ControlRequest>, ctx: egui::Context, folder: PathBuf, on_screen: Box<dyn Fn() -> Ready>) -> McpHost {
        if let Err(e) = std::fs::create_dir_all(&folder) {
            log::warn!("mcp: {}: {e}", folder.display());
        }
        McpHost { control, ctx, control_port: None, endpoint: None, failed: None, folder, lan: None, lan_at: None, on_screen }
    }

    /// The page for the stored settings (a JSON string); off unless it is to start with the app.
    pub fn page(stored: Option<String>) -> McpServerPage {
        let mut settings: McpServerSettings = stored.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        settings.enabled &= settings.at_start;
        McpServerPage::new(settings)
    }

    /// The settings to store.
    pub fn stored(app: &SolveApp) -> Option<String> {
        app.mcp_server.as_ref().and_then(|p| serde_json::to_string(&p.settings).ok())
    }

    /// Each frame: start, restart or stop the endpoint as the page says, and tell it what runs.
    pub fn tick(&mut self, app: &mut SolveApp) {
        let Some(page) = app.mcp_server.as_mut() else { return };
        if std::mem::take(&mut page.new_token) || page.settings.token.is_empty() {
            match new_token() {
                Some(t) => page.settings.token = t,
                None => log::error!("mcp: no random token"),
            }
        }
        let wanted =
            page.settings.enabled.then(|| Wanted { network: page.settings.network, port: page.settings.port, token: page.settings.token.clone() });
        if self.endpoint.as_ref().map(|(_, w)| w) != wanted.as_ref() {
            if let Some((mut endpoint, _)) = self.endpoint.take() {
                endpoint.stop();
                log::info!("mcp: stopped");
            }
            if let Some(w) = wanted.as_ref().filter(|w| self.failed.as_ref().is_none_or(|(f, _)| f != *w)) {
                self.failed = None;
                match self.start(w) {
                    Ok(endpoint) => {
                        log::info!("mcp: listening on port {} ({})", endpoint.port(), if w.network { "network" } else { "this device" });
                        self.endpoint = Some((endpoint, w.clone()));
                    }
                    Err(e) => {
                        log::warn!("mcp: {e}");
                        self.failed = Some((w.clone(), e));
                    }
                }
            }
        }
        if wanted.is_none() {
            self.failed = None;
        }
        if page.settings.network && self.lan_at.is_none_or(|at| at.elapsed() >= LAN_EVERY) {
            self.lan = lan_address().map(|ip| ip.to_string());
            self.lan_at = Some(Instant::now());
        }
        let status = &mut page.status;
        status.listening = self.endpoint.as_ref().map(|(e, _)| e.port());
        status.error = self.failed.as_ref().map(|(_, e)| e.clone());
        status.lan = self.lan.clone();
        status.folder = self.folder.to_string_lossy().into_owned();
        let activity = self.endpoint.as_ref().map(|(e, _)| e.activity()).unwrap_or_default();
        status.sessions = activity.sessions;
        status.requests = activity.requests;
        status.last_request = activity.last_request;
        status.refused = activity.refused;
    }

    fn start(&mut self, w: &Wanted) -> Result<Endpoint, String> {
        let control_port = match self.control_port {
            Some(p) => p,
            None => {
                // The desktop app's own control server, as it listens by default: a request still
                // running after a minute is answered "running" and the MCP server waits for it
                // through `control.pending` (and tells the agent to call `pending` after a few
                // minutes more), so a long boolean never looks like an app that stopped
                // answering. Its requests join the app's.
                let requests = crate::control_server::start(solidcraft_engine::control_auth::Listen::Exact(0), self.ctx.clone());
                let (forward, ctx) = (self.control.clone(), self.ctx.clone());
                std::thread::Builder::new()
                    .name("mcp-control".into())
                    .spawn(move || {
                        for request in requests {
                            if forward.send(request).is_err() {
                                break;
                            }
                            ctx.request_repaint();
                        }
                    })
                    .map_err(|e| e.to_string())?;
                let p = solidcraft_engine::control_auth::read_current_port().ok_or_else(|| tr("mcp.no_control").to_string())?;
                self.control_port = Some(p);
                p
            }
        };
        let folders = vec![self.folder.clone()];
        let make: MakeServer = std::sync::Arc::new(move || {
            let remote = solidcraft_mcp::Remote::new(&format!("127.0.0.1:{control_port}"), None);
            solidcraft_mcp::Server::new(Box::new(remote)).with_path_scope(Some(solidcraft_mcp::agent_scope_with(None, folders.clone())))
        });
        let ip = if w.network { Ipv4Addr::UNSPECIFIED } else { Ipv4Addr::LOCALHOST };
        Endpoint::start(SocketAddr::from((ip, w.port)), w.token.clone(), make, (self.on_screen)()).map_err(|e| {
            let port = w.port.to_string();
            if e.kind() == std::io::ErrorKind::AddrInUse {
                format("mcp.port_in_use", &[("port", &port)])
            } else {
                format("mcp.start_failed", &[("port", &port), ("error", &e.to_string())])
            }
        })
    }
}
