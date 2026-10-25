//! Gear: spur, helical, herringbone, internal, rack, bevel, miter, worm, worm wheel and crossed
//! helical gears through `gear.create`, and Edit Gear (`gear.edit`) on a gear already made. The
//! rows change with the type; empty fields take the engine's defaults, shown as hints. What the
//! gear comes to (diameters, twist, cone angles, lead, the pair's centre distance) is worked out
//! by `gear.info` as the values change, with any warning or the reason it can't be made.

use serde_json::{Map, Value, json};
use solidcraft_engine::{GEAR_TYPES, Sel};

use crate::SolveApp;
use crate::dialogs::{Kind, form_row};
use crate::i18n::tr;
use crate::selection::SelInput;
use crate::ui_kit::{checkbox_grid, choice_row, hint_row, section, segmented, text_row};

/// The dialog's state: every field as typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gr {
    pub ty: usize,
    /// Sized by diametral pitch rather than module.
    pub by_dp: bool,
    /// Field text by parameter name.
    pub fields: std::collections::BTreeMap<String, String>,
    pub left: bool,
    pub keyway: bool,
    pub name: String,
    /// Editing this gear component (Edit Gear) rather than making a new one.
    pub component: Option<u64>,
    /// The last `gear.info` answer and the parameters it was for.
    pub info: Option<(Value, Result<Value, String>)>,
}

/// Optional fields by section: (parameter, label, hint).
const TOOTH: [(&str, &str, &str); 5] = [
    ("profile_shift", "Profile shift", "0"),
    ("backlash", "Backlash", "0 mm"),
    ("addendum", "Addendum", "1"),
    ("clearance", "Root clearance", "0.25"),
    ("root_fillet", "Root fillet", "0.38"),
];
const HUB: [(&str, &str, &str); 3] = [("bore", "Bore", "none"), ("hub_diameter", "Hub diameter", "none"), ("hub_length", "Hub length", "none")];

/// The readouts `gear.info` gives, in the order shown: (key, label, unit).
const READOUTS: [(&str, &str, &str); 26] = [
    ("pitch_diameter", "Pitch diameter", "mm"),
    ("tip_diameter", "Tip diameter", "mm"),
    ("root_diameter", "Root diameter", "mm"),
    ("base_diameter", "Base diameter", "mm"),
    ("outside_diameter", "Outside diameter", "mm"),
    ("circular_pitch", "Circular pitch", "mm"),
    ("axial_pitch", "Axial pitch", "mm"),
    ("tooth_thickness", "Tooth thickness", "mm"),
    ("transverse_module", "Transverse module", "mm"),
    ("transverse_pressure_angle", "Transverse pressure angle", "deg"),
    ("twist", "Twist", "deg"),
    ("lead", "Lead", "mm"),
    ("lead_angle", "Lead angle", "deg"),
    ("turns", "Turns", ""),
    ("pitch_cone_angle", "Pitch cone angle", "deg"),
    ("face_angle", "Face angle", "deg"),
    ("root_angle", "Root angle", "deg"),
    ("cone_distance", "Cone distance", "mm"),
    ("virtual_teeth", "Virtual teeth", ""),
    ("length", "Length", "mm"),
    ("tooth_height", "Tooth height", "mm"),
    ("keyway", "Keyway", ""),
    ("ratio", "Ratio", ""),
    ("centre_distance", "Center distance", "mm"),
    ("contact_ratio", "Contact ratio", ""),
    ("overlap_ratio", "Overlap ratio", ""),
];

impl Gr {
    fn new() -> Self {
        Gr::default()
    }

    fn type_id(&self) -> &'static str {
        GEAR_TYPES.get(self.ty).map_or("spur", |t| t.0)
    }

    fn field(&mut self, key: &str) -> &mut String {
        self.fields.entry(key.to_string()).or_default()
    }

    /// The gear's command parameters (empty fields left out: the engine's defaults).
    pub fn params(&self) -> Value {
        let ty = self.type_id();
        let mut p = Map::new();
        p.insert("type".into(), json!(ty));
        for (k, v) in &self.fields {
            let text = v.trim();
            if text.is_empty() || !self.uses(k) || (k == "module" && self.by_dp) || (k == "diametral_pitch" && !self.by_dp) {
                continue;
            }
            p.insert(k.clone(), json!(text));
        }
        if self.uses("hand") {
            p.insert("hand".into(), json!(if self.left { "left" } else { "right" }));
        }
        if self.uses("keyway") && self.keyway {
            p.insert("keyway".into(), json!(true));
        }
        if !self.name.trim().is_empty() {
            p.insert("name".into(), json!(self.name.trim()));
        }
        Value::Object(p)
    }

    /// Whether this type has the parameter.
    fn uses(&self, key: &str) -> bool {
        let ty = self.type_id();
        let round = ty != "rack";
        match key {
            "module" | "diametral_pitch" | "teeth" | "pressure_angle" | "face_width" | "name" => true,
            "profile_shift" | "backlash" | "addendum" | "clearance" | "root_fillet" => true,
            "helix_angle" => matches!(ty, "helical" | "herringbone" | "crossed_helical" | "internal" | "rack" | "bevel"),
            "hand" => matches!(ty, "helical" | "herringbone" | "crossed_helical" | "internal" | "rack" | "bevel" | "worm" | "worm_wheel"),
            "bore" | "keyway" | "hub_diameter" | "hub_length" => round && ty != "internal",
            "rim" => ty == "internal",
            "height" => ty == "rack",
            "mate_teeth" => matches!(ty, "bevel" | "spur" | "helical" | "herringbone"),
            "shaft_angle" => ty == "bevel",
            "starts" | "worm_diameter" => matches!(ty, "worm" | "worm_wheel"),
            _ => false,
        }
    }

    /// The dialog of a gear already made, from what it was made from.
    fn restore(component: u64, stored: &Value) -> Self {
        let mut g = Gr::new();
        g.component = Some(component);
        let ty = stored.get("type").and_then(Value::as_str).unwrap_or("spur");
        g.ty = GEAR_TYPES.iter().position(|t| t.0 == ty).unwrap_or(0);
        let mm = ["module", "face_width", "backlash", "bore", "hub_diameter", "hub_length", "rim", "height", "worm_diameter"];
        let deg = ["pressure_angle", "helix_angle", "shaft_angle"];
        if let Some(o) = stored.as_object() {
            for (k, v) in o {
                let Some(x) = v.as_f64() else { continue };
                // Zero means none for the bore and hub; leave those fields empty.
                if matches!(k.as_str(), "bore" | "hub_diameter" | "hub_length") && x == 0.0 {
                    continue;
                }
                let text = if mm.contains(&k.as_str()) {
                    format!("{} mm", trim(x))
                } else if deg.contains(&k.as_str()) {
                    format!("{} deg", trim(x))
                } else {
                    trim(x)
                };
                g.fields.insert(k.clone(), text);
            }
        }
        g.left = stored.get("hand").and_then(Value::as_str) == Some("left");
        g.keyway = stored.get("keyway").and_then(Value::as_bool).unwrap_or(false);
        g
    }

    /// What the gear comes to, worked out again only when a value changed.
    fn info(&mut self, app: &mut SolveApp) -> Result<Value, String> {
        let mut p = self.params();
        if let Some(mate) = self.fields.get("mate_teeth").map(|s| s.trim()).filter(|s| !s.is_empty()) {
            p["mate_teeth"] = json!(mate);
        }
        if let Some((k, v)) = &self.info
            && *k == p
        {
            return v.clone();
        }
        let r = app.session.execute("gear.info", &p).map_err(|e| e.to_string());
        self.info = Some((p, r.clone()));
        r
    }
}

fn trim(x: f64) -> String {
    let s = format!("{x:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A gear component: the one picked in the browser, or whose bodies are selected.
pub fn selected_gear(app: &SolveApp) -> Option<u64> {
    let gear = |id: u64| solidcraft_engine::stored_gear(&app.session, id).is_some().then_some(id);
    if app.session.selection.is_empty() {
        let [id] = app.tree.picked_components.as_slice() else { return None };
        return gear(*id);
    }
    let st = app.session.world_state();
    let mut found = None;
    for sel in &app.session.selection {
        let id = match sel {
            Sel::Body { name } | Sel::Face { body: name, .. } | Sel::Edge { body: name, .. } | Sel::Vertex { body: name, .. } => {
                let b = st.bodies.iter().find(|b| b.name == *name)?;
                app.session.doc.model_body_component(b)
            }
            Sel::Occurrence { path } => app.session.occurrence_placement(path).ok()?.component,
            _ => return None,
        };
        if found.is_some_and(|f| f != id) {
            return None;
        }
        found = Some(id);
    }
    found.and_then(gear)
}

/// Whether a component is a gear.
pub fn is_gear(app: &SolveApp, component: u64) -> bool {
    solidcraft_engine::stored_gear(&app.session, component).is_some()
}

/// The dialogs for Gear and Edit Gear.
pub fn start(app: &SolveApp, id: &str) -> Option<(Kind, Vec<SelInput>)> {
    match id {
        "gear.create" => Some((Kind::Gear(Gr::new()), vec![])),
        "gear.edit" => {
            let comp = selected_gear(app)?;
            let stored = solidcraft_engine::stored_gear(&app.session, comp)?;
            Some((Kind::Gear(Gr::restore(comp, &stored)), vec![]))
        }
        _ => None,
    }
}

/// Edit Gear on this component (the browser's double-click and the context menu).
pub fn open(app: &mut SolveApp, component: u64) {
    if !is_gear(app, component) {
        app.set_status("Select a gear to edit it", true);
        return;
    }
    app.session.selection.clear();
    app.tree.picked_components = vec![component];
    app.start("gear.edit");
}

fn rows_for(ui: &mut egui::Ui, k: &mut Gr, list: &[(&str, &str, &str)]) -> bool {
    let mut enter = false;
    for (key, label, hint) in list {
        if k.uses(key) {
            enter |= hint_row(ui, label, k.field(key), tr(hint));
        }
    }
    enter
}

/// The dialog's rows. Returns Enter pressed.
pub fn rows(app: &mut SolveApp, ui: &mut egui::Ui, k: &mut Gr) -> bool {
    let mut enter = false;
    let names: Vec<&str> = GEAR_TYPES.iter().map(|t| t.1).collect();
    let before = k.ty;
    choice_row(ui, "Type", "gear_type", &mut k.ty, &names);
    if k.ty != before {
        // What only the old type had (and its helix angle and face) start over.
        for key in ["helix_angle", "face_width"] {
            k.fields.remove(key);
        }
    }
    let ty = k.type_id();
    form_row(ui, "Size by", |ui| {
        let mut by = usize::from(k.by_dp);
        if segmented(ui, &mut by, &["Module", "Diametral pitch"], Default::default()) {
            k.by_dp = by == 1;
        }
    });
    enter |= if k.by_dp {
        hint_row(ui, "Diametral pitch", k.field("diametral_pitch"), "e.g. 24")
    } else {
        hint_row(ui, "Module", k.field("module"), "1 mm")
    };
    let (teeth_label, teeth_hint) = match ty {
        "worm" => ("Starts", "1"),
        "rack" => ("Teeth", "10"),
        "internal" => ("Teeth", "60"),
        "worm_wheel" => ("Teeth", "30"),
        _ => ("Teeth", "20"),
    };
    enter |= hint_row(ui, teeth_label, k.field(if ty == "worm" { "starts" } else { "teeth" }), teeth_hint);
    enter |= hint_row(ui, "Pressure angle", k.field("pressure_angle"), "20 deg");
    if k.uses("helix_angle") {
        let (label, hint) = match ty {
            "bevel" => ("Spiral angle", "0 deg"),
            "helical" | "herringbone" => ("Helix angle", "20 deg"),
            "crossed_helical" => ("Helix angle", "45 deg"),
            _ => ("Helix angle", "0 deg"),
        };
        enter |= hint_row(ui, label, k.field("helix_angle"), hint);
    }
    if k.uses("hand") {
        form_row(ui, "Hand", |ui| {
            let mut hand = usize::from(k.left);
            if segmented(ui, &mut hand, &["Right", "Left"], Default::default()) {
                k.left = hand == 1;
            }
        });
    }
    let auto_face = k.info.as_ref().and_then(|(_, r)| r.as_ref().ok()).and_then(|i| i["face_width"].as_f64());
    let face_hint = auto_face.map(|f| format!("{} {}", tr("auto"), crate::prefs::show_mm(app, f, 1))).unwrap_or_else(|| tr("auto").to_string());
    enter |= hint_row(ui, if ty == "worm" { "Length" } else { "Face width" }, k.field("face_width"), &face_hint);
    match ty {
        "internal" => enter |= hint_row(ui, "Rim", k.field("rim"), "3 module"),
        "rack" => enter |= hint_row(ui, "Back height", k.field("height"), "2 module"),
        "bevel" => {
            enter |= hint_row(ui, "Mate teeth", k.field("mate_teeth"), tr("same as teeth"));
            enter |= hint_row(ui, "Shaft angle", k.field("shaft_angle"), "90 deg");
        }
        "worm" => enter |= hint_row(ui, "Worm diameter", k.field("worm_diameter"), "10 module"),
        "worm_wheel" => {
            enter |= hint_row(ui, "Worm starts", k.field("starts"), "1");
            enter |= hint_row(ui, "Worm diameter", k.field("worm_diameter"), "10 module");
        }
        _ => {}
    }
    section(ui, "gear_tooth", "Tooth form", false, |ui| enter |= rows_for(ui, k, &TOOTH));
    if k.uses("bore") {
        section(ui, "gear_hub", "Bore and hub", k.fields.get("bore").is_some_and(|s| !s.trim().is_empty()), |ui| {
            enter |= rows_for(ui, k, &HUB);
            checkbox_grid(ui, &mut [("Keyway (DIN 6885)", &mut k.keyway)], 2);
        });
    }
    if matches!(ty, "spur" | "helical" | "herringbone") {
        section(ui, "gear_mate", "Mating gear", false, |ui| {
            enter |= hint_row(ui, "Mate teeth", k.field("mate_teeth"), tr("none"));
        });
    }
    let _ = text_row(ui, "Name", &mut k.name, tr("automatic"));
    let info = k.info(app);
    section(ui, "gear_dims", "Dimensions", true, |ui| match &info {
        Ok(v) => {
            let t = crate::theme::Tokens::get();
            for (key, label, unit) in READOUTS {
                let Some(x) = v.get(key) else { continue };
                let text = match (x.as_f64(), unit) {
                    (Some(n), "mm") => crate::prefs::show_mm(app, n, 1),
                    (Some(n), "deg") => format!("{n:.3}°"),
                    (Some(n), _) => format!("{n:.3}"),
                    (None, _) => x.as_str().unwrap_or_default().to_string(),
                };
                form_row(ui, label, |ui| crate::ui_kit::text(ui, &text, crate::ui_kit::Text::Cell));
            }
            for w in v["warnings"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                ui.add(egui::Label::new(egui::RichText::new(w).size(t.font_small).color(t.warning)).wrap());
            }
        }
        Err(why) => {
            // The whole reason, wrapped: a touch screen has no hover to show a cut-off one.
            let t = crate::theme::Tokens::get();
            // Without the command's name ("invalid parameters for `gear.info`: …").
            let why = why.split_once("`: ").map_or(why.as_str(), |(_, rest)| rest);
            ui.add(egui::Label::new(egui::RichText::new(why).size(t.font_small).color(t.warning)).wrap());
        }
    });
    enter
}

/// The commands OK runs.
pub fn commands(k: &Gr) -> Result<Vec<(String, Value)>, String> {
    if let Some((p, Err(why))) = &k.info
        && *p == k.params()
    {
        return Err(why.clone());
    }
    let mut p = k.params();
    match k.component {
        Some(c) => {
            p["component"] = json!(c);
            Ok(vec![("gear.edit".into(), p)])
        }
        None => Ok(vec![("gear.create".into(), p)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty fields are left to the engine; other types' fields aren't sent.
    #[test]
    fn params_follow_the_type() {
        let mut g = Gr::new();
        *g.field("module") = "2 mm".into();
        *g.field("teeth") = " 24 ".into();
        *g.field("helix_angle") = "30 deg".into();
        *g.field("rim") = "5 mm".into();
        *g.field("face_width") = String::new();
        assert_eq!(g.params(), json!({"type": "spur", "module": "2 mm", "teeth": "24"}));
        g.ty = GEAR_TYPES.iter().position(|t| t.0 == "helical").unwrap_or(0);
        g.left = true;
        assert_eq!(g.params(), json!({"type": "helical", "module": "2 mm", "teeth": "24", "helix_angle": "30 deg", "hand": "left"}));
        g.by_dp = true;
        *g.field("diametral_pitch") = "16".into();
        assert_eq!(g.params()["diametral_pitch"], "16");
        assert!(g.params().get("module").is_none());
    }

    /// Edit Gear opens with what the gear was made from, and OK edits that component.
    #[test]
    fn edit_restores_the_stored_parameters() {
        let mut s = solidcraft_engine::Session::default();
        let r = s.execute("gear.create", &json!({"type": "worm_wheel", "module": 1.5, "teeth": 40, "starts": 2, "bore": 8})).unwrap_or_default();
        let comp = r["component"].as_u64().unwrap_or(0);
        let stored = solidcraft_engine::stored_gear(&s, comp).unwrap_or_default();
        let g = Gr::restore(comp, &stored);
        assert_eq!(g.type_id(), "worm_wheel");
        let p = g.params();
        assert_eq!(
            (p["module"].as_str(), p["teeth"].as_str(), p["starts"].as_str(), p["bore"].as_str()),
            (Some("1.5 mm"), Some("40"), Some("2"), Some("8 mm"))
        );
        assert!(p.get("hub_diameter").is_none(), "no hub: {p}");
        let cmds = commands(&g).unwrap_or_default();
        assert_eq!(cmds.first().map(|c| (c.0.as_str(), c.1["component"].as_u64())), Some(("gear.edit", Some(comp))));
        // Rebuilding with the same values gives the same gear.
        let mut e = s.scratch();
        let again = e.execute("gear.edit", &cmds.first().map(|c| c.1.clone()).unwrap_or_default());
        assert!(again.is_ok(), "{again:?}");
        assert_eq!(solidcraft_engine::stored_gear(&e, comp), Some(stored));
    }
}
