//! The first local appearance-library consumer, inside the existing Appearance panel.
#[cfg(not(target_arch = "wasm32"))]
use crate::dialogs_appearance::selected_params;
use crate::{SolveApp, dialogs_appearance::Ap, selection::SelInput, ui_kit};
use serde_json::json;
use solidcraft_engine::io::catalog::Catalog;
#[cfg(not(target_arch = "wasm32"))]
use solidcraft_engine::io::catalog::Definition;
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct LocalLibrary {
    /// A delayed picker may update only the editor that requested it.
    token: Arc<()>,
    pub search: String,
    pub path: String,
    pub catalog: Option<Catalog>,
    pub selected: Option<String>,
    pub overwrite: bool,
}

fn display_order(mut catalog: Catalog) -> Catalog {
    catalog.entries.sort_by(|a, b| a.definition.name().cmp(b.definition.name()).then_with(|| a.id.cmp(&b.id)));
    catalog
}

fn editor<'a>(app: &'a mut SolveApp, token: &Arc<()>) -> Option<&'a mut Ap> {
    let crate::dialogs::Kind::Appearance(k) = &mut app.dialog.as_mut()?.kind else { return None };
    Arc::ptr_eq(&k.local.token, token).then_some(k)
}

fn save_library(app: &mut SolveApp, token: Arc<()>, overwrite: bool) {
    let origin = app.session.doc.clone();
    app.pick_save("my-library.solidcraft-library.json", &["json"], move |app, path| {
        let Some(path) = path else { return };
        if !Arc::ptr_eq(&origin, &app.session.doc) {
            app.set_status("The library export document changed while choosing its destination", true);
            return;
        }
        match app.run("library.export", json!({"path":path,"overwrite":overwrite})) {
            Ok(result) => {
                let parsed = result.get("catalog").cloned().and_then(|v| serde_json::from_value::<Catalog>(v).ok()).map(display_order);
                let count = parsed.as_ref().map_or(0, |c| c.entries.len());
                if let Some(k) = editor(app, &token) {
                    k.local.path = path;
                    k.local.catalog = parsed;
                    k.local.selected = None;
                    k.local.search.clear();
                    k.note = Some(format!("Saved {count} reusable definitions. The design is unchanged."));
                }
            }
            Err(error) => {
                if let Some(k) = editor(app, &token) {
                    k.note = Some(error);
                }
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn open_library(app: &mut SolveApp, token: Arc<()>) {
    app.pick_open(move |app, path| {
        let Some(path) = path else { return };
        let Some(k) = editor(app, &token) else { return };
        let mut local = k.local.clone();
        local.path = path;
        local.catalog = None;
        local.selected = None;
        let note = load(app, &mut local).unwrap_or_else(|error| error);
        if let Some(k) = editor(app, &token) {
            k.local = local;
            k.note = Some(note);
        }
    });
}

/// Exports the design's library to `path` and shows it as the local library.
fn export(app: &mut SolveApp, k: &mut Ap, path: String, overwrite: bool) {
    k.note = Some(match app.run("library.export", json!({"path":path,"overwrite":overwrite})) {
        Ok(result) => {
            let parsed = result.get("catalog").cloned().and_then(|v| serde_json::from_value::<Catalog>(v).ok()).map(display_order);
            let count = parsed.as_ref().map_or(0, |c| c.entries.len());
            k.local.path = path;
            k.local.catalog = parsed;
            k.local.selected = None;
            k.local.search.clear();
            format!("Saved {count} reusable definitions. The design is unchanged.")
        }
        Err(error) => error,
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn load(app: &mut SolveApp, library: &mut LocalLibrary) -> Result<String, String> {
    let result = app.run("library.inspect", json!({"path":library.path}))?;
    let catalog: Catalog = serde_json::from_value(result.get("catalog").cloned().ok_or("Library response is missing")?).map_err(|e| e.to_string())?;
    let note = format!("Opened {} ({} entries)", catalog.name, catalog.entries.len());
    library.catalog = Some(display_order(catalog));
    library.selected = None;
    library.search.clear();
    Ok(note)
}

pub fn rows(app: &mut SolveApp, ui: &mut egui::Ui, k: &mut Ap, inputs: &[SelInput]) {
    ui_kit::section(ui, "local_library", "Local library", true, |ui| {
        #[cfg(not(target_arch = "wasm32"))]
        import_rows(app, ui, k, inputs);
        #[cfg(target_arch = "wasm32")]
        {
            let _ = inputs;
            ui_kit::text(
                ui,
                crate::i18n::tr_label("Open local libraries in the desktop app. You can export this design's library here."),
                ui_kit::Text::Note("library_platform"),
            );
        }
        ui_kit::checkbox(ui, crate::i18n::tr_label("Replace existing library file"), &mut k.local.overwrite);
        let save = ui_kit::button(ui, crate::i18n::tr_label("Export design library..."), ui_kit::Button::Normal);
        crate::scenario::publish_handle("library:export", save.rect.center());
        if save.clicked() {
            if app.services.pick_save.is_some() {
                save_library(app, k.local.token.clone(), k.local.overwrite);
            } else if !k.local.path.is_empty() {
                export(app, k, k.local.path.clone(), k.local.overwrite);
            }
        }
        ui_kit::text(
            ui,
            crate::i18n::tr_label("Apply your appearance first. Exports assigned appearances and materials created in this design."),
            ui_kit::Text::Note("library_export"),
        );
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn import_rows(app: &mut SolveApp, ui: &mut egui::Ui, k: &mut Ap, inputs: &[SelInput]) {
    let path = ui_kit::row(ui, "Library file", |ui| {
        let id = ui.make_persistent_id("library_path");
        ui_kit::cell_editor(ui, id, &mut k.local.path, "Library file", false)
    });
    crate::scenario::publish_handle("library:path", path.rect.center());
    if path.changed() {
        k.local.catalog = None;
        k.local.selected = None;
    }
    ui.horizontal(|ui| {
        if ui_kit::button(ui, crate::i18n::tr_label("Open library..."), ui_kit::Button::Normal).clicked() {
            open_library(app, k.local.token.clone());
        }
        let load_button = ui_kit::button(ui, "Load library", ui_kit::Button::Enabled(!k.local.path.is_empty()));
        crate::scenario::publish_handle("library:load", load_button.rect.center());
        if load_button.clicked() {
            k.local.catalog = None;
            k.local.selected = None;
            k.note = Some(load(app, &mut k.local).unwrap_or_else(|error| error));
        }
    });
    if let Some(catalog) = &k.local.catalog {
        ui_kit::text(ui, &catalog.name, ui_kit::Text::Report("library_name", false));
        let search = ui_kit::row(ui, "Search this library", |ui| {
            let id = ui.make_persistent_id("library_search");
            ui_kit::search_field(ui, id, &mut k.local.search, "Search this library")
        });
        crate::scenario::publish_handle("library:search", search.rect.center());
        if search.changed() {
            k.local.selected = None;
        }
        let query = k.local.search.trim().to_lowercase();
        let appearances: Vec<_> = catalog
            .entries
            .iter()
            .filter(|entry| matches!(&entry.definition, Definition::Appearance(_)) && entry.definition.name().to_lowercase().contains(&query))
            .collect();
        if appearances.is_empty() {
            ui_kit::text(ui, crate::i18n::tr_label("No matching appearance presets."), ui_kit::Text::Note("library_empty"));
        }
        let row_height = ui_kit::row_height();
        ui_kit::list_frame(ui, "local_library_entries", |ui| {
            egui::ScrollArea::vertical().id_salt("local_library_entries").max_height(row_height * 5.0).show_rows(
                ui,
                row_height,
                appearances.len(),
                |ui, range| {
                    for index in range {
                        let Some(entry) = appearances.get(index) else { continue };
                        if let Definition::Appearance(look) = &entry.definition {
                            ui.horizontal(|ui| {
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                                let [r, g, b] = look.color;
                                ui_kit::color_thumbnail(ui, ui.id().with((&entry.id, "sample")), egui::Color32::from_rgb(r, g, b), &look.name, ());
                                let button = ui_kit::list_picker(ui, &entry.id, &look.name, k.local.selected.as_deref() == Some(entry.id.as_str()))
                                    .on_hover_text(format!(
                                        "Opacity {:.0}% · {} · {}",
                                        look.opacity * 100.0,
                                        entry.provenance.license,
                                        entry.provenance.source_url.as_deref().unwrap_or("Local definition")
                                    ));
                                crate::scenario::publish_handle(&format!("library:entry:{index}"), button.rect.center());
                                if button.clicked() {
                                    k.local.selected = Some(entry.id.clone());
                                }
                            });
                        }
                    }
                },
            );
        });
    }
    let entry = k
        .local
        .catalog
        .as_ref()
        .and_then(|catalog| {
            catalog
                .entries
                .iter()
                .find(|entry| Some(&entry.id) == k.local.selected.as_ref() && matches!(&entry.definition, Definition::Appearance(_)))
        })
        .cloned();
    let picked = inputs.first().map(|input| input.items.as_slice()).unwrap_or(&[]);
    let selected = selected_params(k, picked);
    let apply = ui_kit::button(ui, "Apply library appearance", ui_kit::Button::Enabled(entry.is_some() && selected.is_some()));
    crate::scenario::publish_handle("library:apply", apply.rect.center());
    if apply.clicked()
        && let (Some(entry), Some(selected)) = (entry, selected)
    {
        let mut targets = serde_json::Map::new();
        for key in ["bodies", "faces", "components"] {
            if let Some(value) = selected.get(key) {
                targets.insert(key.into(), value.clone());
            }
        }
        k.note = Some(match app.run("library.apply", json!({"path":k.local.path,"entry":entry.id,"revision":entry.revision,"targets":targets})) {
            Ok(_) => {
                if let Definition::Appearance(look) = entry.definition {
                    k.color = look.color;
                    k.pbr = None;
                    k.opacity = look.opacity;
                    k.look = Some(look.name.clone());
                    k.remove_override = false;
                    k.applied = selected_params(k, picked);
                    format!("{} copied into this design", look.name)
                } else {
                    "This panel applies appearance presets only".into()
                }
            }
            Err(error) => error,
        });
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "dialogs_library_tests.rs"]
mod tests;
