//! The viewport's frame-rate readout (Preferences ▸ Graphics ▸ "Frame rate").

use egui::{FontId, Rect, pos2, vec2};

use crate::SolveApp;
use crate::theme::Tokens;

/// Weight of the newest frame in the running mean (roughly the last 20 frames count).
const NEWEST: f64 = 0.05;

/// A gap between frames longer than this (seconds) means the view stood still: nothing changed,
/// so nothing was drawn.
const IDLE_AFTER: f64 = 1.0;

/// Frames per second, averaged over recent frames.
#[derive(Clone, Copy, Debug, Default)]
pub struct FrameRate {
    mean_dt: f64,
    /// The last gap between frames was a pause: the view is idle.
    idle: bool,
}

impl FrameRate {
    /// Takes the time since the previous frame, in seconds. A gap over a second means the view
    /// was idle (it draws only when something changes): the mean starts again from the next
    /// frames. Anything that isn't a positive finite time is left out.
    pub fn record(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if dt > IDLE_AFTER {
            *self = FrameRate { mean_dt: 0.0, idle: true };
            return;
        }
        self.mean_dt = if self.mean_dt > 0.0 { self.mean_dt + NEWEST * (dt - self.mean_dt) } else { dt };
        self.idle = false;
    }

    /// The averaged rate; `None` before the first frame was recorded and while the view is idle.
    pub fn fps(&self) -> Option<f64> {
        (self.mean_dt > 0.0).then(|| 1.0 / self.mean_dt)
    }

    /// The readout's text: rate and the matching time per frame, or 0 while the view is idle.
    pub fn label(&self) -> Option<String> {
        if self.idle {
            return Some("0 FPS".to_string());
        }
        self.fps().map(|fps| format!("{fps:.0} FPS · {:.1} ms", 1000.0 * self.mean_dt))
    }
}

/// Records this frame and paints the readout in the view's bottom-right corner. The readout asks
/// for no frames of its own, so it shows how often the view is really drawn and leaves an idle
/// window idle (a readout that redrew every frame kept the GPU busy on an untouched window). Once
/// the frames stop, one more frame a little over [`IDLE_AFTER`] later shows the view idle.
pub(crate) fn paint(app: &mut SolveApp, ctx: &egui::Context, painter: &egui::Painter, rect: Rect) {
    if !app.preferences.show_frame_rate {
        app.frame_rate = FrameRate::default();
        return;
    }
    app.frame_rate.record(f64::from(ctx.input(|i| i.unstable_dt)));
    if app.frame_rate.fps().is_some() {
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(IDLE_AFTER + 0.1));
    }
    let Some(text) = app.frame_rate.label() else { return };
    let t = Tokens::get();
    let font = FontId::monospace(crate::theme::Tokens::get().font_body);
    let galley = painter.layout_no_wrap(text, font, t.text);
    let size = galley.size() + vec2(16.0, 8.0);
    let r = Rect::from_min_size(pos2(rect.right() - 12.0 - size.x, rect.bottom() - 12.0 - size.y), size);
    painter.rect(r, t.radius_sm, t.overlay, t.control_border_stroke(t.border), egui::StrokeKind::Inside);
    crate::scenario::publish_target("viewport:frame_rate", r);
    painter.galley(r.center() - galley.size() / 2.0, galley, t.text);
}

#[cfg(test)]
mod tests {
    use super::FrameRate;

    #[test]
    fn steady_frames_read_their_rate() {
        let mut f = FrameRate::default();
        assert_eq!(f.fps(), None);
        for _ in 0..200 {
            f.record(1.0 / 60.0);
        }
        assert!((f.fps().unwrap_or(0.0) - 60.0).abs() < 1e-6);
        assert_eq!(f.label().as_deref(), Some("60 FPS · 16.7 ms"));
    }

    #[test]
    fn hostile_times_are_left_out_and_a_pause_reads_idle() {
        let mut f = FrameRate::default();
        for dt in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            f.record(dt);
        }
        assert_eq!(f.fps(), None);
        assert_eq!(f.label(), None);
        f.record(0.01);
        assert!((f.fps().unwrap_or(0.0) - 100.0).abs() < 1e-6);
        // Nothing drawn for 3 s: the view was idle, and the next frames start a new mean.
        f.record(3.0);
        assert_eq!(f.fps(), None);
        assert_eq!(f.label().as_deref(), Some("0 FPS"));
        f.record(1.0 / 60.0);
        assert!((f.fps().unwrap_or(0.0) - 60.0).abs() < 1e-6);
    }

    /// The readout must not keep an untouched window drawing: with it on, a frame asks for the
    /// next one only after the idle gap (to show 0 FPS), and the idle frame asks for none.
    #[test]
    fn the_readout_asks_for_no_frames_of_its_own() {
        let mut app = crate::SolveApp::new(solidcraft_engine::Session::default(), crate::Services::default());
        app.preferences.show_frame_rate = true;
        let ctx = crate::fonts::test_context();
        let mut frame = |time: f64| {
            let raw = egui::RawInput { time: Some(time), ..Default::default() };
            let mut out = ctx.run_ui(raw, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter().clone();
                super::paint(&mut app, ui.ctx(), &painter, rect);
            });
            out.textures_delta.clear();
            out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.repaint_delay)
        };
        for i in 0..10 {
            frame(f64::from(i) / 60.0);
        }
        let busy = frame(10.0 / 60.0);
        assert!(busy.is_some_and(|d| d.as_secs_f64() > 1.0), "a running readout asked for a frame after {busy:?}");
        let idle = frame(2.0);
        assert!(idle.is_some_and(|d| d.as_secs_f64() > 3600.0), "an idle readout asked for a frame after {idle:?}");
        assert_eq!(app.frame_rate.label().as_deref(), Some("0 FPS"));
    }

    #[test]
    fn the_mean_follows_a_slower_view() {
        let mut f = FrameRate::default();
        f.record(1.0 / 120.0);
        for _ in 0..300 {
            f.record(1.0 / 30.0);
        }
        assert!((f.fps().unwrap_or(0.0) - 30.0).abs() < 0.1);
    }
}
