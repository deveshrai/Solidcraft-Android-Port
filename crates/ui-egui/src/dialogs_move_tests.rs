//! The Move window's main value: the one the canvas box shows and the keyboard types into.
//! It follows the move type, because each type uses a different value.
use serde_json::json;
use solidcraft_engine::Session;

use super::*;
use crate::Services;
use crate::dialogs::Dialog;

fn move_dialog(mode: usize) -> Dialog {
    let mut s = Session::default();
    s.execute("solid.box", &json!({"length": 20, "width": 20, "height": 20})).unwrap();
    let mut app = SolveApp::new(s, Services::default());
    app.start("solid.move");
    let mut d = app.dialog.clone().unwrap();
    d.kind = kind(mode);
    d.inputs = inputs(mode);
    if let Some(inp) = d.inputs.first_mut() {
        inp.items = vec![Sel::Body { name: "Body1".into() }];
    }
    d
}

#[test]
fn the_main_value_follows_the_move_type() {
    for mode in [FREE, TRANSLATE, POINT_TO_POSITION] {
        let mut d = move_dialog(mode);
        let got = d.primary().map(|p| (p.0, p.1));
        assert_eq!(got, Some(("X", ValueKind::Length)), "{} types a distance", TYPES[mode]);
    }

    let mut d = move_dialog(ROTATE);
    let got = d.primary().map(|p| (p.0, p.1));
    assert_eq!(got, Some(("Angle", ValueKind::Angle)), "Rotate types an angle, not an X distance");

    let mut d = move_dialog(POINT_TO_POINT);
    assert!(d.primary().is_none(), "Point to Point is two picks with nothing to type");
}

#[test]
fn the_main_value_writes_through_to_the_command() {
    let mut d = move_dialog(ROTATE);
    if let Some((_, _, value)) = d.primary() {
        *value = "90 deg".into();
    }
    let Kind::Move { angle, .. } = &d.kind else { panic!("not a move") };
    assert_eq!(angle, "90 deg", "the box edits the angle itself, not a copy of it");

    let mut d = move_dialog(TRANSLATE);
    if let Some((_, _, value)) = d.primary() {
        *value = "30 mm".into();
    }
    let Kind::Move { x, y, z, .. } = &d.kind else { panic!("not a move") };
    assert_eq!((x.as_str(), y.as_str(), z.as_str()), ("30 mm", "0 mm", "0 mm"), "the box edits X alone");
}

/// Free Move and Translate carry the triad. The other three have no manipulator yet, so their
/// value box is the only way to drive them: it must still have somewhere to sit.
#[test]
fn every_move_type_has_somewhere_to_put_its_value_box() {
    let mut s = Session::default();
    s.execute("solid.box", &json!({"length": 20, "width": 20, "height": 20})).unwrap();
    let app = SolveApp::new(s, Services::default());
    for mode in [FREE, TRANSLATE, ROTATE, POINT_TO_POINT, POINT_TO_POSITION] {
        let d = move_dialog(mode);
        let has_triad = triad(&app, &d).is_some();
        assert_eq!(has_triad, matches!(mode, FREE | TRANSLATE), "{} triad", TYPES[mode]);
        let at = triad(&app, &d).map(|t| t.center).or_else(|| crate::canvas::anchor(&app, &d).map(|(base, _)| base));
        assert!(at.is_some(), "{} has nowhere to put its value box", TYPES[mode]);
    }
}

/// Point to Point takes the origin point as From or To (#3696): a click on the origin dot picks
/// it, and the body moves from its corner onto the origin.
#[test]
fn point_to_point_takes_the_origin_point() {
    let mut h = crate::scenario::Harness::new();
    h.app.ui.show_origin = true;
    h.app.run("solid.box", json!({"length": 20, "width": 20, "height": 20})).unwrap();
    h.app.run("solid.move", json!({"bodies": ["Body1"], "translate": ["30 mm", "20 mm", "0 mm"]})).unwrap();
    h.app.cam.distance = 200.0;
    h.settle();
    let hi = h.app.session.model.state().body("Body1").unwrap().mesh().bounds().max;
    h.app.start("solid.move");
    {
        let d = h.app.dialog.as_mut().unwrap();
        d.kind = kind(POINT_TO_POINT);
        d.inputs = inputs(POINT_TO_POINT);
        d.inputs[0].items = vec![Sel::Body { name: "Body1".into() }];
        d.active = 1;
    }
    let corner = h.call("ui.at", json!({ "world": [hi.x, hi.y, hi.z] }))["result"].clone();
    h.step(&json!({ "click": corner })).unwrap();
    h.app.dialog.as_mut().unwrap().active = 2;
    let origin = h.call("ui.at", json!({ "world": [0, 0, 0] }))["result"].clone();
    h.step(&json!({ "click": origin })).unwrap();
    let d = h.app.dialog.clone().unwrap();
    assert_eq!(d.inputs[2].items, vec![Sel::Vertex { body: String::new(), point: Vec3::ZERO }], "the origin is the To point");
    let p = params(&h.app, &d).unwrap();
    assert_eq!(p["translate"], json!([mm(-hi.x), mm(-hi.y), mm(-hi.z)]), "the corner goes onto the origin");
}

/// #4776: the box shows the value of the handle dragged: the distance along the Y arrow after
/// dragging it, the angle of a ring after turning it; typing then edits that value.
#[test]
fn the_box_shows_the_dragged_axis() {
    let mut s = Session::default();
    s.execute("solid.box", &json!({"length": 20, "width": 20, "height": 20})).unwrap();
    let app = SolveApp::new(s, Services::default());
    let mut d = move_dialog(FREE);
    let g = |handle, change| crate::gizmo::Drag { handle, change, started: true, done: true };
    drag(&app, &mut d, g(crate::gizmo::Handle::Axis(1), Change::Translate(Vec3::new(0.0, 5.0, 0.0))));
    let (label, kind, value) = d.primary().unwrap();
    assert_eq!((label, kind, value.as_str()), ("Y", ValueKind::Length, "5 mm"));
    *value = "7 mm".into();
    let Kind::Move { x, y, .. } = &d.kind else { panic!("not a move") };
    assert_eq!((x.as_str(), y.as_str()), ("0 mm", "7 mm"), "typing edits Y");
    drag(&app, &mut d, g(crate::gizmo::Handle::Ring(2), Change::Rotate { axis: 2, angle: 0.5 }));
    assert_eq!(d.primary().map(|p| (p.0, p.1)), Some(("Z Angle", ValueKind::Angle)));
    // Translate has no rings: a ring index past its values leaves the box on a distance.
    let mut t = move_dialog(TRANSLATE);
    drag(&app, &mut t, g(crate::gizmo::Handle::Axis(2), Change::Translate(Vec3::new(0.0, 0.0, 3.0))));
    assert_eq!(t.primary().map(|p| p.0), Some("Z"));
}

/// #4776: Tab in the box goes to the next value (X, Y, Z, then the angles in Free Move) and
/// Shift+Tab back, the keyboard staying in the box; Enter moves the body by what was typed.
#[test]
fn tab_in_the_box_goes_to_the_next_axis() {
    let mut h = crate::scenario::Harness::new();
    h.app.run("solid.box", json!({"length": 20, "width": 20, "height": 20})).unwrap();
    h.settle();
    h.app.run("select.set", json!({"items": [Sel::Body { name: "Body1".into() }]})).unwrap();
    h.app.start("solid.move");
    h.frames(3);
    assert_eq!(h.app.dialog.as_mut().and_then(|d| d.primary()).map(|p| p.0), Some("X"));
    h.step(&json!({"key": "Tab"})).unwrap();
    h.frames(2);
    assert_eq!(h.app.dialog.as_mut().and_then(|d| d.primary()).map(|p| p.0), Some("Y"));
    h.step(&json!({"key": "Tab"})).unwrap();
    h.step(&json!({"key": "Tab", "shift": true})).unwrap();
    h.frames(2);
    assert_eq!(h.app.dialog.as_mut().and_then(|d| d.primary()).map(|p| p.0), Some("Y"), "Shift+Tab goes back");
    h.step(&json!({"text": "12"})).unwrap();
    h.frames(1);
    h.step(&json!({"key": "Enter"})).unwrap();
    h.settle();
    let b = h.app.session.model.state().body("Body1").unwrap().mesh().bounds();
    assert!((b.min.y - 12.0).abs() < 1e-6 && b.min.x.abs() < 1e-6, "moved along Y: {b:?}");
}
