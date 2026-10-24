use crate::ui_kit::test_support::*;
use crate::ui_kit::*;

use crate::{scenario, theme::Tokens};
use egui::Ui;

#[test]
fn row_height_and_label_column_are_fixed_for_long_labels_and_expressions() {
    let ctx = crate::fonts::test_context();
    let mut value = "width / 2 + 10 mm".to_string();
    for _ in 0..3 {
        frame(&ctx, vec![], |ui| {
            row(ui, "A long parameter label that must elide", |ui| {
                field(ui, &mut value);
            });
            row(ui, "Radius", |ui| {
                field(ui, &mut value);
            });
        });
    }
    let first = scenario::rect_of("dialog:row:A long parameter label that must elide").unwrap();
    let second = scenario::rect_of("dialog:row:Radius").unwrap();
    let label = scenario::rect_of("dialog:label:Radius").unwrap();
    let control = scenario::rect_of("dialog:control:Radius").unwrap();
    assert_eq!(first.height(), Tokens::get().control_height);
    assert_eq!(second.height(), Tokens::get().control_height);
    // The design's grid: 28 pt controls with a 5 pt row gap and a 70 pt label column (wider only for long labels).
    assert!(
        (second.top() - first.bottom() - Tokens::get().form_gap_y).abs() < 0.01,
        "Plex Mono fields must retain the row gap: {first:?}, {second:?}"
    );
    // The long label widens the column for every row, up to 1.6 times the design's width.
    assert_eq!(label.width(), Tokens::get().form_label_width * 1.6);
    assert_eq!(scenario::rect_of("dialog:label:A long parameter label that must elide").unwrap().width(), label.width());
    assert!(control.left() > label.right());
    assert!(second.contains_rect(control));
    assert_eq!(value, "width / 2 + 10 mm");
}

#[test]
fn section_toggles_persist_per_dialog() {
    let ctx = crate::fonts::test_context();
    let mut a = 0;
    let mut b = 0;
    let mut draw = |ui: &mut Ui| {
        ui.push_id("fillet", |ui| {
            section(ui, "opts", "Options", false, |_| a += 1);
        });
        ui.push_id("hole", |ui| {
            section(ui, "opts", "Hole options", false, |_| b += 1);
        });
    };
    frame(&ctx, vec![], &mut draw);
    let pos = scenario::handle_at("dialog:section:Options").unwrap();
    frame(&ctx, pointer(pos, true), &mut draw);
    frame(&ctx, pointer(pos, false), &mut draw);
    frame(&ctx, vec![], &mut draw);
    assert!(a >= 2);
    assert_eq!(b, 0);
    let before = a;
    frame(&ctx, pointer(pos, true), |ui| {
        ui.push_id("fillet", |ui| {
            section(ui, "opts", "Options", false, |_| a += 1);
        });
    });
    frame(&ctx, pointer(pos, false), |ui| {
        ui.push_id("fillet", |ui| {
            section(ui, "opts", "Options", false, |_| a += 1);
        });
    });
    assert_eq!(a, before + 1);
}

#[test]
fn short_labels_keep_the_designs_label_column_and_long_ones_widen_it_for_every_row() {
    fn widths(labels: [&str; 2]) -> (f32, f32) {
        let ctx = egui::Context::default();
        crate::fonts::setup_fonts(&ctx);
        let mut value = String::new();
        for _ in 0..3 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_width(300.0);
                scope(ui, |ui| {
                    for label in labels {
                        row(ui, label, |ui| {
                            field(ui, &mut value);
                        });
                    }
                });
            });
            output.textures_delta.clear();
        }
        (
            scenario::rect_of(&format!("dialog:label:{}", labels[0])).unwrap().width(),
            scenario::rect_of(&format!("dialog:label:{}", labels[1])).unwrap().width(),
        )
    }
    let t = Tokens::get();
    let (a, b) = widths(["Edges", "Radius"]);
    assert_eq!((a, b), (t.form_label_width, t.form_label_width));
    let (a, b) = widths(["Edges", "Counterbore depth"]);
    assert_eq!(a, b, "one column per form");
    assert!(a > t.form_label_width && a <= t.form_label_width * 1.6, "{a}");
}
