use crate::ui_kit::test_support::*;
use crate::ui_kit::*;

use crate::scenario;
use egui::Ui;

#[test]
fn segmented_click_changes_selection_and_keeps_all_choices_visible() {
    let ctx = crate::fonts::test_context();
    let mut selected = 0;
    let mut draw = |ui: &mut Ui| {
        row(ui, "Continuity", |ui| segmented(ui, &mut selected, &["G1", "G2", "G3"], Default::default()));
    };
    frame(&ctx, vec![], &mut draw);
    let pos = scenario::handle_at("dialog:Continuity:G2").unwrap();
    frame(&ctx, pointer(pos, true), &mut draw);
    frame(&ctx, pointer(pos, false), &mut draw);
    assert_eq!(selected, 1);
    let control = scenario::rect_of("dialog:control:Continuity").unwrap();
    for option in ["G1", "G2", "G3"] {
        assert!(control.contains(scenario::handle_at(&format!("dialog:Continuity:{option}")).unwrap()));
    }
}

#[test]
fn optional_field_keeps_empty_default_and_options_use_two_columns() {
    let ctx = crate::fonts::test_context();
    let mut value = String::new();
    let (mut flip, mut copy) = (false, true);
    frame(&ctx, vec![], |ui| {
        row(ui, "Depth", |ui| hint_field(ui, &mut value, "to the next face"));
        options(ui, "options", &mut [("Flip", &mut flip), ("Create Copy", &mut copy)]);
    });
    assert!(value.is_empty(), "displaying a hint must not introduce a parameter");
    let first = scenario::handle_at("dialog:Flip").unwrap();
    let second = scenario::handle_at("dialog:Create Copy").unwrap();
    assert_eq!(first.y, second.y);
    assert!(second.x > first.x);
    let mut draw = |ui: &mut Ui| options(ui, "options", &mut [("Flip", &mut flip), ("Create Copy", &mut copy)]);
    frame(&ctx, vec![], &mut draw);
    let first = scenario::handle_at("dialog:Flip").unwrap();
    frame(&ctx, pointer(first, true), &mut draw);
    frame(&ctx, pointer(first, false), &mut draw);
    assert!(flip && copy);
}
