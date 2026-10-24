//! Text elements for dense cards; see `docs/ui-kit.md` for tokens and usage.
use crate::theme::Tokens;
use egui::Ui;

/// Paint one pixel-aligned text run with the requested interface weight.
/// Clipped, device-pixel-snapped weighted text. Tokens: `font_section_tracking; caller supplies font/ink tokens`.
pub(super) fn paint_text(ui: &Ui, rect: egui::Rect, text: &str, font: egui::FontId, ink: egui::Color32, spaced: bool) {
    let size = font.size;
    let mut format = egui::TextFormat::simple(font, ink);
    if spaced {
        format.extra_letter_spacing = size * Tokens::get().font_section_tracking;
    }
    let mut job = egui::text::LayoutJob::single_section(text.to_string(), format);
    job.wrap.max_width = rect.width().max(0.0);
    job.wrap.max_rows = 1;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    // Snap to whole device pixels: a fractional origin blurs every glyph edge.
    let ppp = ui.ctx().pixels_per_point().max(0.5);
    let snap = |v: f32| (v * ppp).round() / ppp;
    let at = egui::pos2(snap(rect.left()), snap(rect.center().y - galley.size().y / 2.0));
    let painter = ui.painter().with_clip_rect(ui.clip_rect().intersect(rect));
    painter.galley(at, galley, ink);
}
