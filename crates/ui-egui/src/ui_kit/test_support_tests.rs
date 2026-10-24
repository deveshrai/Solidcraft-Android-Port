use super::*;
use crate::scenario;
use egui::{Ui, vec2};

pub(crate) use crate::test_support::{key, pointer};

fn draw_frame(ctx: &egui::Context, events: Vec<egui::Event>, mut add: impl FnMut(&mut Ui)) -> egui::FullOutput {
    scenario::clear_handles();
    crate::test_support::frame(
        ctx,
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(320.0, 400.0))),
            events,
            focused: true,
            ..Default::default()
        },
        |ui| {
            ui.set_width(280.0);
            scope(ui, &mut add);
        },
    )
}

pub(crate) fn frame(ctx: &egui::Context, events: Vec<egui::Event>, add: impl FnMut(&mut Ui)) {
    draw_frame(ctx, events, add);
}

/// The cursor the frame asks the platform for: the pointer is at `pos` for the whole frame.
pub(crate) fn frame_cursor(ctx: &egui::Context, pos: egui::Pos2, add: impl FnMut(&mut Ui)) -> egui::CursorIcon {
    draw_frame(ctx, vec![egui::Event::PointerMoved(pos)], add).platform_output.cursor_icon
}
