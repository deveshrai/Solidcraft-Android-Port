//! Settings › MCP server (Android port): let Claude Code, or any MCP client, drive the app over
//! the network, from Termux on the same device or from a computer on the same Wi-Fi.
//!
//! The page only shows and edits [`McpServerPage`]; the host that offers it (the Android app)
//! runs the server, reads the settings each frame and writes back what the server is doing.
//! Hosts that don't set `SolveApp::mcp_server` show no such page.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::*;
use crate::i18n::tr;

/// The port the server listens on unless the user picks another.
pub const DEFAULT_PORT: u16 = 27880;
/// The name the server goes by in the client's configuration.
pub const SERVER_NAME: &str = "solidcraft";

/// What the user chose; the host keeps it across launches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct McpServerSettings {
    /// The server is running.
    pub enabled: bool,
    pub port: u16,
    /// Listen on the network (Wi-Fi), not just for apps on this device.
    pub network: bool,
    /// Turn it on when the app starts, as it was left.
    pub at_start: bool,
    /// What a client must send as `Authorization: Bearer <token>`.
    pub token: String,
}

impl Default for McpServerSettings {
    fn default() -> Self {
        McpServerSettings { enabled: false, port: DEFAULT_PORT, network: false, at_start: false, token: String::new() }
    }
}

/// What the server is doing; the host writes it every frame.
#[derive(Clone, Debug, Default)]
pub struct McpServerStatus {
    /// The port it listens on, once it does.
    pub listening: Option<u16>,
    /// Why it isn't (the port is taken, …).
    pub error: Option<String>,
    /// This device's address on the network, when it has one.
    pub lan: Option<String>,
    /// The folder the agent may read and write files in.
    pub folder: String,
    /// MCP sessions open (clients connected).
    pub sessions: usize,
    pub requests: u64,
    pub last_request: Option<Instant>,
    /// Requests refused for a missing or wrong token.
    pub refused: u64,
}

/// The page's state: the settings, the server's status and what the page asks the host for.
#[derive(Debug, Default)]
pub struct McpServerPage {
    pub settings: McpServerSettings,
    pub status: McpServerStatus,
    /// A new token was asked for (Advanced); the host makes one and clears this.
    pub new_token: bool,
    port_text: String,
    show_token: bool,
    copied: Option<(&'static str, Instant)>,
}

impl McpServerPage {
    pub fn new(settings: McpServerSettings) -> Self {
        McpServerPage { port_text: settings.port.to_string(), settings, ..Default::default() }
    }
}

/// The address a client uses: this device's own for an app on it (Termux), else its network one.
pub fn url(host: &str, port: u16) -> String {
    format!("http://{host}:{port}/mcp")
}

/// One line that starts Claude Code connected to the server, for this session only.
pub fn launch_command(url: &str, token: &str) -> String {
    // Written out rather than through serde_json, which would sort the keys: type, url, then the
    // token, as a person reads it. Neither the URL nor the hex token has a character JSON escapes.
    let json = |s: &str| serde_json::Value::from(s).to_string();
    format!(
        "claude --mcp-config '{{\"mcpServers\":{{\"{SERVER_NAME}\":{{\"type\":\"http\",\"url\":{},\"headers\":{{\"Authorization\":{}}}}}}}}}'",
        json(url),
        json(&format!("Bearer {token}"))
    )
}

/// One line that adds the server to Claude Code for good (then plain `claude` has it).
pub fn add_command(url: &str, token: &str) -> String {
    format!("claude mcp add --transport http {SERVER_NAME} {url} --header \"Authorization: Bearer {token}\"")
}

/// A port typed into the field, when it is one a server may listen on.
fn parse_port(text: &str) -> Option<u16> {
    text.trim().parse::<u16>().ok().filter(|p| *p >= 1024)
}

/// Seconds since `t`, as the status line says it.
fn ago(t: Instant) -> String {
    let s = t.elapsed().as_secs();
    if s < 60 { format!("{s} s") } else { format!("{} min", s / 60) }
}

/// The page in Settings.
pub(super) fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    let Some(mut page) = app.mcp_server.take() else { return };
    draw(&mut page, ui);
    app.mcp_server = Some(page);
}

/// A copyable block of text: the text, wrapped and selectable, with a Copy button under it.
fn copy_block(ui: &mut egui::Ui, page: &mut McpServerPage, key: &'static str, text: &str) {
    let t = Tokens::get();
    egui::Frame::NONE
        .fill(t.control_fill)
        .stroke(t.control_border_stroke(t.control_border))
        .corner_radius(t.control_radius)
        .inner_margin(egui::Margin::symmetric(t.form_inset as i8, t.space_6 as i8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(egui::Label::new(egui::RichText::new(text).font(egui::FontId::monospace(t.font_body)).color(t.form_ink)).wrap().selectable(true));
        });
    let copied = page.copied.is_some_and(|(k, at)| k == key && at.elapsed().as_secs() < 3);
    let label = if copied { tr("mcp.copied") } else { tr("mcp.copy") };
    let response = ui_kit::button(ui, label, ui_kit::Button::Normal);
    crate::scenario::publish_handle(&format!("prefs:mcp:copy:{key}"), response.rect.center());
    if response.clicked() {
        ui.ctx().copy_text(text.to_string());
        page.copied = Some((key, Instant::now()));
    }
    if copied {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    }
}

fn draw(page: &mut McpServerPage, ui: &mut egui::Ui) {
    let t = Tokens::get();
    let labels = t.space_24 * 6.0 + t.space_6;
    ui_kit::settings_heading(ui, tr("prefs.mcp_server"), tr("mcp.intro"));

    ui_kit::settings_section(ui, "mcp_server", tr("mcp.server"), |ui| {
        let response = ui_kit::checkbox(ui, tr("mcp.enable"), &mut page.settings.enabled);
        crate::scenario::publish_handle("prefs:mcp:enable", response.rect.center());
        if !page.settings.enabled {
            ui_kit::text(ui, tr("mcp.off_note"), ui_kit::Text::Hint("mcp_off"));
            return;
        }
        ui_kit::settings_block(ui, tr("mcp.connect_from"), labels, |ui| {
            let mut from = usize::from(page.settings.network);
            if ui_kit::segmented(ui, &mut from, &[tr("mcp.this_device"), tr("mcp.other_computer")], Default::default()) {
                page.settings.network = from == 1;
            }
        });
        let host = if page.settings.network { page.status.lan.clone() } else { Some("127.0.0.1".to_string()) };
        let port = page.status.listening.unwrap_or(page.settings.port);
        match (&page.status.error, page.status.listening, &host) {
            (Some(error), _, _) => {
                ui_kit::text(ui, error, ui_kit::Text::Error("mcp_error"));
            }
            (None, Some(_), Some(host)) => {
                let url = url(host, port);
                ui_kit::text(ui, tr("mcp.url"), ui_kit::Text::Heading("mcp_url"));
                copy_block(ui, page, "url", &url);
                ui_kit::text(ui, tr("mcp.command"), ui_kit::Text::Heading("mcp_command"));
                let token = page.settings.token.clone();
                copy_block(ui, page, "command", &launch_command(&url, &token));
                let note = if page.settings.network { tr("mcp.command_note_network") } else { tr("mcp.command_note_device") };
                ui_kit::text(ui, note, ui_kit::Text::Hint("mcp_command_note"));
            }
            (None, Some(_), None) => {
                ui_kit::text(ui, tr("mcp.no_network"), ui_kit::Text::Error("mcp_no_network"));
            }
            (None, None, _) => {
                ui_kit::text(ui, tr("mcp.starting"), ui_kit::Text::Hint("mcp_starting"));
            }
        }
        let status = if page.status.listening.is_none() {
            String::new()
        } else if page.status.sessions == 0 {
            tr("mcp.waiting").to_string()
        } else {
            let last = page.status.last_request.map(ago).unwrap_or_default();
            crate::i18n::format("mcp.connected", &[("n", &page.status.sessions.to_string()), ("ago", &last)])
        };
        if !status.is_empty() {
            ui_kit::text(ui, &status, ui_kit::Text::Note("mcp_status"));
            if page.status.last_request.is_some() {
                ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
            }
        }
        if page.status.refused > 0 {
            let refused = crate::i18n::format("mcp.refused", &[("n", &page.status.refused.to_string())]);
            ui_kit::text(ui, &refused, ui_kit::Text::Report("mcp_refused", true));
        }
    });

    if !page.settings.enabled {
        return;
    }
    ui_kit::settings_section(ui, "mcp_advanced", tr("mcp.advanced"), |ui| {
        ui_kit::settings_block_hint(ui, tr("mcp.port"), tr("mcp.port_note"), labels, |ui| {
            ui.horizontal(|ui| {
                let (enter, _) =
                    ui_kit::settings_control_width(ui, t.space_24 * 4.0, |ui| ui_kit::text_field(ui, &mut page.port_text, &DEFAULT_PORT.to_string()));
                let valid = parse_port(&page.port_text);
                let changed = valid.is_some_and(|p| p != page.settings.port);
                let response = ui_kit::button(ui, tr("mcp.use_port"), ui_kit::Button::Enabled(changed));
                if changed && (enter || response.clicked()) {
                    page.settings.port = valid.unwrap_or(DEFAULT_PORT);
                }
            });
            if parse_port(&page.port_text).is_none() {
                ui_kit::text(ui, tr("mcp.port_invalid"), ui_kit::Text::Report("mcp_port_invalid", true));
            }
        });
        ui_kit::text(ui, tr("mcp.add_command"), ui_kit::Text::Heading("mcp_add_command"));
        if let (Some(host), Some(port)) =
            (if page.settings.network { page.status.lan.clone() } else { Some("127.0.0.1".into()) }, page.status.listening)
        {
            let token = page.settings.token.clone();
            copy_block(ui, page, "add", &add_command(&url(&host, port), &token));
        }
        ui_kit::settings_block_hint(ui, tr("mcp.token"), tr("mcp.token_note"), labels, |ui| {
            ui.horizontal(|ui| {
                let shown = if page.show_token { page.settings.token.clone() } else { "•".repeat(12) };
                ui_kit::text(ui, &shown, ui_kit::Text::MonoCell);
            });
            ui.horizontal(|ui| {
                let label = if page.show_token { tr("mcp.hide") } else { tr("mcp.show") };
                if ui_kit::button(ui, label, ui_kit::Button::Normal).clicked() {
                    page.show_token = !page.show_token;
                }
                if ui_kit::button(ui, tr("mcp.new_token"), ui_kit::Button::Normal).clicked() {
                    page.new_token = true;
                }
            });
        });
        ui_kit::checkbox(ui, tr("mcp.at_start"), &mut page.settings.at_start);
        if !page.status.folder.is_empty() {
            ui_kit::settings_block_hint(ui, tr("mcp.folder"), tr("mcp.folder_note"), labels, |ui| {
                ui_kit::text(ui, &page.status.folder, ui_kit::Text::MonoNote("mcp_folder"));
            });
        }
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_launch_command_is_one_line_claude_code_reads() {
        let url = url("192.168.1.20", 27880);
        assert_eq!(url, "http://192.168.1.20:27880/mcp");
        let cmd = launch_command(&url, "abc123");
        assert!(!cmd.contains('\n'));
        let json = cmd.strip_prefix("claude --mcp-config '").and_then(|c| c.strip_suffix('\'')).unwrap();
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(v["mcpServers"]["solidcraft"]["type"], "http");
        assert_eq!(v["mcpServers"]["solidcraft"]["url"], url);
        assert_eq!(v["mcpServers"]["solidcraft"]["headers"]["Authorization"], "Bearer abc123");
        assert_eq!(
            add_command(&url, "abc123"),
            "claude mcp add --transport http solidcraft http://192.168.1.20:27880/mcp --header \"Authorization: Bearer abc123\""
        );
    }

    #[test]
    fn only_unprivileged_ports_are_taken() {
        assert_eq!(parse_port(" 8080 "), Some(8080));
        assert_eq!(parse_port("80"), None);
        assert_eq!(parse_port("70000"), None);
        assert_eq!(parse_port("x"), None);
    }

    #[test]
    fn the_page_shows_the_url_and_command_once_listening() {
        let mut h = crate::scenario::Harness::new();
        let mut page = McpServerPage::new(McpServerSettings { enabled: true, token: "feedface".into(), ..Default::default() });
        page.status.listening = Some(DEFAULT_PORT);
        h.app.mcp_server = Some(Box::new(page));
        crate::prefs::open_at(&mut h.app, crate::prefs::PAGE_MCP);
        h.frames(4);
        assert!(crate::scenario::handle_at("prefs:mcp:enable").is_some());
        assert!(crate::scenario::handle_at("prefs:mcp:copy:url").is_some());
        assert!(crate::scenario::handle_at("prefs:mcp:copy:command").is_some());
    }
}
