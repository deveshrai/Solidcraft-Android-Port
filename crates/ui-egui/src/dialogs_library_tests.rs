use super::*;
use crate::{dialogs::Kind, scenario::Harness};
use solidcraft_engine::{Sel, Session};

#[test]
fn catalog_clicked_load_apply_export_and_ok_keep_one_design_undo_step() {
    let mut h = Harness::new();
    h.call("ui.resize", json!({"width":1600,"height":1400}));
    let path = h.dir.join("source-library.json").to_string_lossy().into_owned();
    let mut source = Session::default();
    source.execute("solid.box", &json!({"length":10,"width":10,"height":10})).unwrap();
    source.execute("appearance.assign", &json!({"bodies":["Body1"],"appearance":"Warm oak colour","color":[180,120,70],"opacity":0.8})).unwrap();
    source.execute("library.export", &json!({"path":path})).unwrap();
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10})).unwrap();
    h.app
        .run(
            "appearance.assign",
            json!({"bodies":["Body1"],"color":[20,20,20],
        "pbr":{"metallic":1.0,"roughness":0.3,"dielectric_ior":1.5}}),
        )
        .unwrap();
    h.app.session.mark_saved();
    h.app.session.selection = vec![Sel::Body { name: "Body1".into() }];
    h.app.start("appearance.assign");
    if let Some(dialog) = &mut h.app.dialog
        && let Kind::Appearance(state) = &mut dialog.kind
    {
        state.local.path = path;
    }
    h.frames(3);
    let undo = h.app.session.undo.len();
    h.step(&json!({"widget":"Load library"})).unwrap();
    for handle in [
        "library:path",
        "library:load",
        "library:search",
        "library:entry:0",
        "library:apply",
        "library:export",
        "dialog:button::Open library...",
        "dialog:button::Load library",
        "dialog:button::Apply library appearance",
        "dialog:button::Export design library...",
    ] {
        assert!(crate::scenario::handle_at(handle).is_some(), "missing {handle}");
    }
    for message in
        ["Replace existing library file", "Apply your appearance first. Exports assigned appearances and materials created in this design."]
    {
        assert!(h.widget(message).is_some(), "missing {message}");
    }
    h.step(&json!({"click":{"handle":"library:search"}})).unwrap();
    h.step(&json!({"text":"no such appearance"})).unwrap();
    assert!(h.widget("No matching appearance presets.").is_some());
    h.step(&json!({"key":"A","cmd":true})).unwrap();
    h.step(&json!({"key":"Backspace"})).unwrap();
    h.step(&json!({"click":{"handle":"library:entry:0"}})).unwrap();
    h.step(&json!({"widget":"Apply library appearance"})).unwrap();
    assert_eq!(h.app.session.doc.appearances.bodies["Body1"].name, "Warm oak colour");
    assert!(h.app.session.doc.appearances.bodies["Body1"].pbr.is_none());
    assert_eq!(h.app.session.undo.len(), undo + 1);
    let revision = h.app.session.revision;
    h.step(&json!({"widget":"Export design library..."})).unwrap();
    let exported = h.dir.join("my-library.solidcraft-library.json");
    let catalog = Catalog::read(&exported.to_string_lossy()).unwrap();
    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(catalog.entries[0].definition.name(), "Warm oak colour");
    assert_eq!(h.app.session.revision, revision);
    h.step(&json!({"widget":"OK"})).unwrap();
    assert!(h.app.dialog.is_none());
    assert_eq!(h.app.session.undo.len(), undo + 1);
}

fn appearance_fixture() -> Result<(Harness, Arc<()>), String> {
    let mut h = Harness::new();
    h.app.run("solid.box", json!({"length":10,"width":10,"height":10}))?;
    h.app.run("appearance.assign", json!({"bodies":["Body1"],"appearance":"Warm oak colour","color":[180,120,70]}))?;
    h.app.session.mark_saved();
    let token = start_appearance(&mut h)?;
    Ok((h, token))
}

fn start_appearance(h: &mut Harness) -> Result<Arc<()>, String> {
    h.app.start("appearance.assign");
    match h.app.dialog.as_ref().map(|dialog| &dialog.kind) {
        Some(Kind::Appearance(k)) => Ok(k.local.token.clone()),
        _ => Err("appearance dialog missing".into()),
    }
}

#[test]
fn async_library_export_holds_its_document_until_the_answer() {
    use crate::picker_test_support::{Slot, save};
    let (mut h, token) = appearance_fixture().unwrap();
    let original = h.app.session.doc.clone();
    let path = h.dir.join("delayed-library.json");
    let ctx = crate::fonts::test_context();
    let slot: Slot = Default::default();
    save(&mut h.app, &slot);
    save_library(&mut h.app, token, false);
    crate::file_picker::tick(&mut h.app, &ctx);
    assert!(!path.exists());
    crate::documents::new_design(&mut h.app);
    let other = h.app.session.doc.clone();
    *slot.borrow_mut() = Some(Some(path.to_string_lossy().into_owned()));
    crate::file_picker::tick(&mut h.app, &ctx);
    assert_eq!(h.app.docs.active, 0, "upstream holds tab changes while a picker is pending");
    assert!(Arc::ptr_eq(&other, &h.app.session.doc));
    let catalog = Catalog::read(&path.to_string_lossy()).unwrap();
    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(catalog.entries[0].definition.name(), "Warm oak colour");
    crate::documents::switch(&mut h.app, 0);
    assert!(Arc::ptr_eq(&original, &h.app.session.doc));
    assert!(!h.app.session.is_dirty());
}

#[test]
fn async_library_export_captures_overwrite_consent_and_cancel_writes_nothing() {
    use crate::picker_test_support::{Slot, save};
    let (mut h, token) = appearance_fixture().unwrap();
    let path = h.dir.join("existing-library.json");
    std::fs::write(&path, b"keep this file").unwrap();
    let original = h.app.session.doc.clone();
    let ctx = crate::fonts::test_context();
    let slot: Slot = Default::default();
    save(&mut h.app, &slot);
    save_library(&mut h.app, token.clone(), false);
    crate::file_picker::tick(&mut h.app, &ctx);
    editor(&mut h.app, &token).unwrap().local.overwrite = true;
    *slot.borrow_mut() = Some(Some(path.to_string_lossy().into_owned()));
    crate::file_picker::tick(&mut h.app, &ctx);
    assert_eq!(std::fs::read(&path).unwrap(), b"keep this file");
    assert!(matches!(&h.app.status, Some((_, _, true))));
    save_library(&mut h.app, token, false);
    crate::file_picker::tick(&mut h.app, &ctx);
    *slot.borrow_mut() = Some(None);
    crate::file_picker::tick(&mut h.app, &ctx);
    assert!(!h.app.file_picker_busy());
    assert_eq!(std::fs::read(&path).unwrap(), b"keep this file");
    assert!(Arc::ptr_eq(&original, &h.app.session.doc));
}

#[test]
fn async_library_open_does_not_replace_a_new_appearance_editor() {
    use crate::picker_test_support::{Slot, save};
    let (mut h, token) = appearance_fixture().unwrap();
    let path = h.dir.join("source-library.json");
    h.app.run("library.export", json!({"path":path})).unwrap();
    let original = h.app.session.doc.clone();
    let ctx = crate::fonts::test_context();
    let slot: Slot = Default::default();
    save(&mut h.app, &slot);
    // Prepare a different editor before the picker holds user commands, then model
    // an internal host replacing it while the old asynchronous answer is pending.
    let original_editor = h.app.dialog.take();
    let newer = start_appearance(&mut h).unwrap();
    editor(&mut h.app, &newer).unwrap().local.path = "new editor draft".into();
    let replacement_editor = h.app.dialog.take();
    h.app.dialog = original_editor;
    crate::picker_test_support::open(&mut h.app, &slot);
    open_library(&mut h.app, token);
    crate::file_picker::tick(&mut h.app, &ctx);
    h.app.dialog = replacement_editor;
    *slot.borrow_mut() = Some(Some(path.to_string_lossy().into_owned()));
    crate::file_picker::tick(&mut h.app, &ctx);
    let k = editor(&mut h.app, &newer).unwrap();
    assert_eq!(k.local.path, "new editor draft");
    assert!(k.local.catalog.is_none());
    assert!(Arc::ptr_eq(&original, &h.app.session.doc));
}

#[test]
fn delayed_library_export_refuses_an_internally_changed_design_without_overwriting() {
    use crate::picker_test_support::{Slot, save};
    let (mut h, token) = appearance_fixture().unwrap();
    let path = h.dir.join("keep.json");
    std::fs::write(&path, b"keep library").unwrap();
    let slot: Slot = Default::default();
    save(&mut h.app, &slot);
    save_library(&mut h.app, token, true);
    h.app.session.execute("appearance.assign", &json!({"bodies":["Body1"],"color":[1,2,3]})).unwrap();
    let changed = h.app.session.doc.clone();
    let history = (h.app.session.undo.len(), h.app.session.revision);
    *slot.borrow_mut() = Some(Some(path.to_string_lossy().into_owned()));
    crate::file_picker::tick(&mut h.app, &crate::fonts::test_context());
    assert_eq!(std::fs::read(path).unwrap(), b"keep library");
    assert!(Arc::ptr_eq(&changed, &h.app.session.doc));
    assert_eq!((h.app.session.undo.len(), h.app.session.revision), history);
    assert!(matches!(&h.app.status, Some((_, _, true))));
}
