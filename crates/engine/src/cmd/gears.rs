//! Gears: spur, helical, herringbone, internal (ring), rack, bevel, miter, worm, worm wheel and
//! crossed helical (screw) gears, generated from their standard relations (`gear_profile`).
//!
//! Each gear is inserted as its own component built by ordinary features: a sketch of the tooth
//! outline (involute flanks as splines, tip and root as arcs), then an extrude (straight teeth),
//! a twisted sweep (helical teeth and worms), a scaled sweep towards the pitch apex (bevel
//! teeth), and the bore, keyway and hub. What it was made from is kept in the component's
//! metadata under [`KEY`], so Edit Gear rebuilds it with other values.
//!
//! Gear frame: the axis is +z from z = 0 (a bevel gear's heel plane; its pitch apex is up the
//! axis), tooth 0 centred on +x. A rack runs along +x with its pitch line on y = 0, teeth up +y.
//! A worm wheel's mid-plane is z = face width / 2, its worm's axis parallel to y at
//! x = centre distance.

use serde_json::{Map, Value, json};
use solidcraft_geom::{Vec2, Vec3};
use solidcraft_io::Geom2;
use std::f64::consts::{PI, TAU};

use super::CommandSpec;
use crate::gear_profile::{self as gp, Rack};
use crate::params::{bad, bool_, num, str_, vec3};
use crate::{EngineError, Result, Session};
use solidcraft_doc::Document;
use solidcraft_doc::expr::Kind;

/// The component metadata key that holds a gear's parameters.
pub const KEY: &str = "io.solidcraft.gear";

const CREATE_DOC: &str = "type: spur | helical | herringbone | internal | rack | bevel | miter | worm | worm_wheel | crossed_helical (screw gear); \
module?: mm (normal module for helical teeth; a worm's axial module; default 1) | diametral_pitch?: teeth per inch of pitch diameter; \
teeth? (default 20; a rack's length in teeth; a worm's starts are `starts`); pressure_angle?: deg (default 20); \
helix_angle?: deg (helical, herringbone, crossed_helical (default 45), internal, rack; bevel: spiral angle, approximate); hand?: right|left (default right); \
face_width?: mm (default 10 module; bevel: a third of the cone distance at most; a worm's length); \
profile_shift?: x (default 0); backlash?: mm at the pitch circle, shared between the pair (default 0); addendum?: 1; clearance?: 0.25; root_fillet?: 0.38 (coefficients of the module); \
bore?: mm (0: none); keyway?: bool (DIN 6885 for the bore); hub_diameter?, hub_length?: mm (a hub behind z = 0); \
internal: rim?: mm outside the root circle (default 3 module); rack: height?: mm below the root (default 2 module); \
bevel / miter: mate_teeth? (default teeth), shaft_angle?: deg (default 90); \
worm: starts? (default 1), worm_diameter?: pitch diameter mm (default 10 module); \
worm_wheel: starts?, worm_diameter? (of its worm); \
name?; point?: [x,y,z] with direction?: [x,y,z] (where the gear's axis goes; default the origin, +z)";

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("gear.create", "Gear", create_atomic).at("SOLID", "CREATE").icon("spur_gear").params(CREATE_DOC),
    CommandSpec::new("gear.info", "Gear Dimensions", info).icon("spur_gear").noundo().params(
        "the gear.create parameters → the gear's dimensions (pitch, tip, root and base diameters, tooth thickness, twist, cone angles, lead, centre distance…) and warnings; \
         mate_teeth?: with a mating gear's teeth, the pair's centre distance and contact ratio",
    ),
    CommandSpec::new("gear.edit", "Edit Gear", edit_atomic)
        .icon("spur_gear")
        .params("component: id|name of a gear; any gear.create parameter: the gear is rebuilt with it (others keep their values)"),
];

/// The kinds of gear.
pub const TYPES: [(&str, &str); 10] = [
    ("spur", "Spur"),
    ("helical", "Helical"),
    ("herringbone", "Herringbone"),
    ("internal", "Internal (ring)"),
    ("rack", "Rack"),
    ("bevel", "Bevel"),
    ("miter", "Miter"),
    ("worm", "Worm"),
    ("worm_wheel", "Worm wheel"),
    ("crossed_helical", "Crossed helical"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Type {
    Spur,
    Helical,
    Herringbone,
    Internal,
    Rack,
    Bevel,
    Miter,
    Worm,
    WormWheel,
    Crossed,
}

impl Type {
    fn parse(s: &str) -> Option<Type> {
        Some(match s.trim().to_ascii_lowercase().replace([' ', '-'], "_").as_str() {
            "spur" | "straight" => Type::Spur,
            "helical" => Type::Helical,
            "herringbone" | "double_helical" => Type::Herringbone,
            "internal" | "ring" => Type::Internal,
            "rack" => Type::Rack,
            "bevel" => Type::Bevel,
            "miter" | "mitre" => Type::Miter,
            "worm" => Type::Worm,
            "worm_wheel" | "wormwheel" | "worm_gear" => Type::WormWheel,
            "crossed_helical" | "crossed" | "screw" => Type::Crossed,
            _ => return None,
        })
    }
    fn id(self) -> &'static str {
        match self {
            Type::Spur => "spur",
            Type::Helical => "helical",
            Type::Herringbone => "herringbone",
            Type::Internal => "internal",
            Type::Rack => "rack",
            Type::Bevel => "bevel",
            Type::Miter => "miter",
            Type::Worm => "worm",
            Type::WormWheel => "worm_wheel",
            Type::Crossed => "crossed_helical",
        }
    }
    fn label(self) -> &'static str {
        TYPES.iter().find(|t| t.0 == self.id()).map_or("Gear", |t| t.1)
    }
    fn helical(self) -> bool {
        matches!(self, Type::Helical | Type::Herringbone | Type::Crossed)
    }
    fn round(self) -> bool {
        !matches!(self, Type::Rack)
    }
}

/// A gear's parameters, checked, in millimetres and radians.
#[derive(Clone, Debug, PartialEq)]
struct Spec {
    ty: Type,
    m: f64,
    teeth: usize,
    alpha: f64,
    helix: f64,
    left: bool,
    face: f64,
    /// The face width was the default for the size (so an edit of the size works it out again).
    face_auto: bool,
    x: f64,
    backlash: f64,
    ha: f64,
    c: f64,
    rho: f64,
    bore: f64,
    keyway: bool,
    hub_d: f64,
    hub_len: f64,
    rim: f64,
    height: f64,
    mate_teeth: usize,
    shaft_angle: f64,
    starts: usize,
    worm_d: f64,
}

const CMD: &str = "gear.create (Gear)";

/// A number, or an expression over the design's parameters (lengths in the design's units unless
/// it says otherwise; angles in degrees). `None` when absent or empty.
fn value(doc: &Document, p: &Value, k: &str, kind: Kind) -> Result<Option<f64>> {
    match p.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(t)) if t.trim().is_empty() => Ok(None),
        Some(Value::String(t)) => {
            let v = doc.eval(t.trim(), kind).map_err(|e| bad(CMD, format!("`{k}`: {e}")))?;
            Ok(Some(if kind == Kind::Angle { v.to_degrees() } else { v }))
        }
        Some(_) => num(p, k).map(Some).ok_or_else(|| bad(CMD, format!("`{k}` must be a number or an expression"))),
    }
}

fn opt(doc: &Document, p: &Value, k: &str, lo: f64, hi: f64, what: &str) -> Result<Option<f64>> {
    let kind = match what {
        "mm" => Kind::Length,
        "degrees" => Kind::Angle,
        _ => Kind::Unitless,
    };
    match value(doc, p, k, kind)? {
        None => Ok(None),
        Some(v) if v.is_finite() && v >= lo && v <= hi => Ok(Some(v)),
        Some(_) => Err(bad(CMD, format!("`{k}` ({what}) must be from {lo} to {hi}"))),
    }
}

fn count(doc: &Document, p: &Value, k: &str, lo: usize, hi: usize) -> Result<Option<usize>> {
    match value(doc, p, k, Kind::Unitless)? {
        None => Ok(None),
        Some(n) if n.is_finite() && (n - n.round()).abs() < 1e-9 && n >= lo as f64 && n <= hi as f64 => Ok(Some(n.round() as usize)),
        Some(_) => Err(bad(CMD, format!("`{k}` must be a whole number from {lo} to {hi}"))),
    }
}

impl Spec {
    fn parse(doc: &Document, p: &Value) -> Result<Spec> {
        let ty = match str_(p, "type") {
            None => Type::Spur,
            Some(t) => Type::parse(t).ok_or_else(|| bad(CMD, format!("unknown gear type `{t}` ({})", TYPES.map(|t| t.0).join(", "))))?,
        };
        let m = match (opt(doc, p, "module", 0.01, 200.0, "mm")?, opt(doc, p, "diametral_pitch", 0.1, 2540.0, "teeth per inch")?) {
            (Some(m), _) => m,
            (None, Some(dp)) => 25.4 / dp,
            (None, None) => 1.0,
        };
        let deg = |k: &str, lo: f64, hi: f64, d: f64| -> Result<f64> { Ok(opt(doc, p, k, lo, hi, "degrees")?.unwrap_or(d).to_radians()) };
        let teeth_default = match ty {
            Type::Internal => 60,
            Type::Rack => 10,
            Type::Worm => 1,
            Type::WormWheel => 30,
            _ => 20,
        };
        let teeth = count(doc, p, "teeth", 1, 1000)?.unwrap_or(teeth_default);
        let helix_default = match ty {
            Type::Helical | Type::Herringbone => 20.0,
            Type::Crossed => 45.0,
            _ => 0.0,
        };
        let helix = deg("helix_angle", 0.0, 60.0, helix_default)?;
        let left = match str_(p, "hand") {
            None => false,
            Some(h) if h.eq_ignore_ascii_case("right") => false,
            Some(h) if h.eq_ignore_ascii_case("left") => true,
            Some(h) => return Err(bad(CMD, format!("`hand` must be right or left, not `{h}`"))),
        };
        let starts = count(doc, p, "starts", 1, 12)?.unwrap_or(1);
        let worm_d = opt(doc, p, "worm_diameter", 0.1, 10_000.0, "mm")?.unwrap_or(10.0 * m);
        let mate_teeth = count(doc, p, "mate_teeth", 3, 1000)?.unwrap_or(teeth);
        let spec = Spec {
            ty,
            m,
            teeth,
            alpha: deg("pressure_angle", 10.0, 35.0, 20.0)?,
            helix,
            left,
            face: opt(doc, p, "face_width", 0.01, 10_000.0, "mm")?.unwrap_or(0.0),
            face_auto: false,
            x: opt(doc, p, "profile_shift", -1.0, 1.5, "a coefficient")?.unwrap_or(0.0),
            backlash: opt(doc, p, "backlash", 0.0, 10.0 * m, "mm")?.unwrap_or(0.0),
            ha: opt(doc, p, "addendum", 0.3, 1.5, "a coefficient")?.unwrap_or(1.0),
            c: opt(doc, p, "clearance", 0.0, 0.6, "a coefficient")?.unwrap_or(0.25),
            rho: opt(doc, p, "root_fillet", 0.0, 0.6, "a coefficient")?.unwrap_or(0.38),
            bore: opt(doc, p, "bore", 0.0, 10_000.0, "mm")?.unwrap_or(0.0),
            keyway: bool_(p, "keyway").unwrap_or(false),
            hub_d: opt(doc, p, "hub_diameter", 0.0, 10_000.0, "mm")?.unwrap_or(0.0),
            hub_len: opt(doc, p, "hub_length", 0.0, 10_000.0, "mm")?.unwrap_or(0.0),
            rim: opt(doc, p, "rim", 0.01, 10_000.0, "mm")?.unwrap_or(3.0 * m),
            height: opt(doc, p, "height", 0.01, 10_000.0, "mm")?.unwrap_or(2.0 * m),
            mate_teeth: if ty == Type::Miter { teeth } else { mate_teeth },
            shaft_angle: if ty == Type::Miter { PI / 2.0 } else { deg("shaft_angle", 10.0, 170.0, 90.0)? },
            starts,
            worm_d,
        };
        let mut spec = spec;
        if spec.face == 0.0 {
            spec.face = spec.default_face();
            spec.face_auto = true;
        }
        if matches!(spec.ty, Type::Worm) {
            spec.teeth = spec.starts;
        }
        let min_teeth = match spec.ty {
            Type::Rack | Type::Worm => 1,
            Type::Internal => 6,
            _ => 3,
        };
        if spec.teeth < min_teeth {
            return Err(bad(CMD, format!("a {} needs at least {min_teeth} teeth", spec.ty.label().to_lowercase())));
        }
        Ok(spec)
    }

    fn default_face(&self) -> f64 {
        match self.ty {
            Type::Bevel | Type::Miter => {
                let (_, ae) = self.cone();
                (ae / 3.0).min(10.0 * self.m)
            }
            Type::Worm => {
                // Long enough for a wheel of 30 teeth or so to engage fully.
                (11.0 + 0.06 * 30.0) * self.m
            }
            Type::WormWheel => 0.75 * self.worm_d,
            Type::Herringbone => 16.0 * self.m,
            _ => 10.0 * self.m,
        }
    }

    /// A bevel gear's pitch cone angle and outer cone distance.
    fn cone(&self) -> (f64, f64) {
        let ratio = self.mate_teeth as f64 / self.teeth as f64;
        let delta = self.shaft_angle.sin().atan2(ratio + self.shaft_angle.cos());
        let r = self.m * self.teeth as f64 / 2.0;
        (delta, r / delta.sin())
    }

    /// The worm's (or the wheel's worm's) lead and lead angle.
    fn lead(&self) -> (f64, f64) {
        let lead = self.starts as f64 * PI * self.m;
        (lead, (lead / (PI * self.worm_d)).atan())
    }

    /// The basic rack in the transverse plane (helical teeth are generated there).
    fn rack(&self) -> Rack {
        let thin = self.backlash / 2.0;
        match self.ty {
            Type::Helical | Type::Herringbone | Type::Crossed | Type::Internal | Type::Rack if self.helix > 0.0 => {
                let cb = self.helix.cos();
                Rack {
                    m: self.m / cb,
                    alpha: (self.alpha.tan() / cb).atan(),
                    ha: self.ha * cb,
                    c: self.c * cb,
                    rho: self.rho * cb,
                    x: self.x * cb,
                    thin: thin / cb,
                }
            }
            _ => Rack { m: self.m, alpha: self.alpha, ha: self.ha, c: self.c, rho: self.rho, x: self.x, thin },
        }
    }

    /// The transverse pitch radius.
    fn pitch_radius(&self) -> f64 {
        self.rack().m * self.teeth as f64 / 2.0
    }

    /// The angle the teeth turn through over the face width (signed: right hand turns
    /// counter-clockwise going up +z).
    fn twist(&self) -> f64 {
        let sign = if self.left { -1.0 } else { 1.0 };
        let t = match self.ty {
            Type::Helical | Type::Crossed | Type::Internal => self.face * self.helix.tan() / self.pitch_radius(),
            Type::Herringbone => self.face / 2.0 * self.helix.tan() / self.pitch_radius(),
            Type::WormWheel => self.face * self.lead().1.tan() / self.pitch_radius(),
            Type::Worm => TAU * self.face / self.lead().0,
            Type::Bevel | Type::Miter => {
                let (_, ae) = self.cone();
                let rm = self.pitch_radius() * (1.0 - self.face / (2.0 * ae));
                self.face * self.helix.tan() / rm
            }
            _ => 0.0,
        };
        sign * t
    }

    fn to_json(&self) -> Value {
        let mut o = Map::new();
        o.insert("type".into(), json!(self.ty.id()));
        o.insert("module".into(), json!(self.m));
        o.insert("teeth".into(), json!(self.teeth));
        o.insert("pressure_angle".into(), json!(self.alpha.to_degrees()));
        o.insert("helix_angle".into(), json!(self.helix.to_degrees()));
        o.insert("hand".into(), json!(if self.left { "left" } else { "right" }));
        // An automatic face width is stored as none: it follows the size when the gear is edited.
        o.insert("face_width".into(), if self.face_auto { Value::Null } else { json!(self.face) });
        o.insert("profile_shift".into(), json!(self.x));
        o.insert("backlash".into(), json!(self.backlash));
        o.insert("addendum".into(), json!(self.ha));
        o.insert("clearance".into(), json!(self.c));
        o.insert("root_fillet".into(), json!(self.rho));
        o.insert("bore".into(), json!(self.bore));
        o.insert("keyway".into(), json!(self.keyway));
        o.insert("hub_diameter".into(), json!(self.hub_d));
        o.insert("hub_length".into(), json!(self.hub_len));
        match self.ty {
            Type::Internal => {
                o.insert("rim".into(), json!(self.rim));
            }
            Type::Rack => {
                o.insert("height".into(), json!(self.height));
            }
            Type::Bevel | Type::Miter => {
                o.insert("mate_teeth".into(), json!(self.mate_teeth));
                o.insert("shaft_angle".into(), json!(self.shaft_angle.to_degrees()));
            }
            Type::Worm | Type::WormWheel => {
                o.insert("starts".into(), json!(self.starts));
                o.insert("worm_diameter".into(), json!(self.worm_d));
            }
            _ => {}
        }
        Value::Object(o)
    }

    fn name(&self) -> String {
        let m = trim(self.m);
        match self.ty {
            Type::Rack => format!("Rack m{m} x{}", self.teeth),
            Type::Worm => format!("Worm m{m} {}-start", self.starts),
            t => {
                let kind = match t {
                    Type::Spur => "Spur Gear",
                    Type::Helical => "Helical Gear",
                    Type::Herringbone => "Herringbone Gear",
                    Type::Internal => "Internal Gear",
                    Type::Bevel => "Bevel Gear",
                    Type::Miter => "Miter Gear",
                    Type::WormWheel => "Worm Wheel",
                    _ => "Crossed Helical Gear",
                };
                format!("{kind} m{m} z{}", self.teeth)
            }
        }
    }
}

fn trim(x: f64) -> String {
    let s = format!("{x:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// DIN 6885-1 parallel keys: (shaft diameter over, up to, key width, depth in the hub).
const KEYS: [(f64, f64, f64, f64); 16] = [
    (6.0, 8.0, 2.0, 1.0),
    (8.0, 10.0, 3.0, 1.4),
    (10.0, 12.0, 4.0, 1.8),
    (12.0, 17.0, 5.0, 2.3),
    (17.0, 22.0, 6.0, 2.8),
    (22.0, 30.0, 8.0, 3.3),
    (30.0, 38.0, 10.0, 3.3),
    (38.0, 44.0, 12.0, 3.3),
    (44.0, 50.0, 14.0, 3.8),
    (50.0, 58.0, 16.0, 4.3),
    (58.0, 65.0, 18.0, 4.4),
    (65.0, 75.0, 20.0, 4.9),
    (75.0, 85.0, 22.0, 5.4),
    (85.0, 95.0, 25.0, 5.4),
    (95.0, 110.0, 28.0, 6.4),
    (110.0, 130.0, 32.0, 7.4),
];

fn key_for(bore: f64) -> Option<(f64, f64)> {
    KEYS.iter().find(|k| bore > k.0 - 1e-9 && bore <= k.1 + 1e-9).map(|k| (k.2, k.3))
}

/// What the gear comes to: its outline and the numbers the dialog shows.
struct Plan {
    /// The outline on the profile sketch (z = 0).
    outline: Vec<Geom2>,
    /// Radius inside which the body is solid all round (for the bore and hub checks).
    solid_r: f64,
    /// A point in the profile's material (picks the profile, its holes with it).
    inside: Vec2,
    info: Value,
    warnings: Vec<String>,
}

fn plan(sp: &Spec) -> std::result::Result<Plan, String> {
    let k = sp.rack();
    let z = sp.teeth as f64;
    let mut warnings = Vec::new();
    let mut info = Map::new();
    let mm = |x: f64| json!((x * 1e6).round() / 1e6);
    let dg = |x: f64| json!((x.to_degrees() * 1e6).round() / 1e6);
    info.insert("type".into(), json!(sp.ty.id()));
    info.insert("name".into(), json!(sp.name()));
    info.insert("module".into(), mm(sp.m));
    info.insert("face_width".into(), mm(sp.face));
    let tooth_warn = |t: &gp::Tooth, w: &mut Vec<String>| {
        if t.undercut {
            w.push("the teeth are undercut (too few teeth for the pressure angle): a positive profile shift avoids it".into());
        }
        if t.pointed {
            w.push("the teeth come to a point; their tips are cut back".into());
        }
    };
    let out = match sp.ty {
        Type::Spur | Type::Helical | Type::Herringbone | Type::Crossed | Type::WormWheel => {
            let r = k.m * z / 2.0;
            let ra = r + (k.ha + k.x) * k.m;
            let t = gp::generate(z, &k, ra)?;
            tooth_warn(&t, &mut warnings);
            info.insert("pitch_diameter".into(), mm(2.0 * r));
            info.insert("tip_diameter".into(), mm(2.0 * t.ra));
            info.insert("root_diameter".into(), mm(2.0 * t.rf));
            info.insert("base_diameter".into(), mm(2.0 * r * k.alpha.cos()));
            info.insert("circular_pitch".into(), mm(PI * sp.m));
            info.insert("tooth_thickness".into(), mm(PI * sp.m / 2.0 + 2.0 * sp.x * sp.m * sp.alpha.tan() - sp.backlash / 2.0));
            if sp.ty.helical() || sp.ty == Type::WormWheel {
                info.insert("transverse_module".into(), mm(k.m));
                info.insert("transverse_pressure_angle".into(), dg(k.alpha));
                info.insert("lead".into(), mm(TAU * r / sp.helix.tan().max(1e-12)));
            }
            if sp.ty == Type::WormWheel {
                let (lead, gamma) = sp.lead();
                info.insert("lead_angle".into(), dg(gamma));
                info.insert("worm_lead".into(), mm(lead));
                info.insert("centre_distance".into(), mm(r + sp.worm_d / 2.0 + sp.x * sp.m));
                info.insert("ratio".into(), json!(z / sp.starts as f64));
            }
            if sp.twist() != 0.0 {
                info.insert("twist".into(), dg(sp.twist()));
            }
            let solid_r = t.rf;
            Plan { outline: gp::outline(&t, sp.teeth, 1.0, None), solid_r, inside: Vec2::ZERO, info: Value::Null, warnings: Vec::new() }
        }
        Type::Internal => {
            let t = gp::internal(z, &k)?;
            let r = k.m * z / 2.0;
            let outer = t.ra + sp.rim;
            let mut o = gp::outline(&t, sp.teeth, 1.0, None);
            // The space's outline is the hole; turn it so a tooth (not a space) is on +x.
            o = o.iter().map(|g| rotate(g, PI / z)).collect();
            o.push(Geom2::Circle(Vec2::ZERO, outer));
            info.insert("pitch_diameter".into(), mm(2.0 * r));
            info.insert("tip_diameter".into(), mm(2.0 * t.rf));
            info.insert("root_diameter".into(), mm(2.0 * t.ra));
            info.insert("outside_diameter".into(), mm(2.0 * outer));
            info.insert("base_diameter".into(), mm(2.0 * r * k.alpha.cos()));
            info.insert("circular_pitch".into(), mm(PI * sp.m));
            if sp.twist() != 0.0 {
                info.insert("twist".into(), dg(sp.twist()));
            }
            Plan { outline: o, solid_r: 0.0, inside: Vec2::new(-(t.ra + outer) / 2.0, 0.0), info: Value::Null, warnings: Vec::new() }
        }
        Type::Rack => {
            let o = gp::rack(&k, sp.teeth, sp.height)?;
            info.insert("length".into(), mm(sp.teeth as f64 * PI * sp.m));
            info.insert("circular_pitch".into(), mm(PI * sp.m));
            info.insert("tooth_height".into(), mm((2.0 * sp.ha + sp.c) * sp.m));
            info.insert("total_height".into(), mm((2.0 * sp.ha + sp.c) * sp.m + sp.height));
            info.insert("pitch_line_height".into(), mm((sp.ha + sp.c - sp.x) * sp.m + sp.height));
            if sp.helix > 0.0 {
                info.insert("helix_angle".into(), dg(sp.helix));
            }
            let inside = Vec2::new(0.0, -(sp.ha + sp.c - sp.x) * sp.m - sp.height / 2.0);
            Plan { outline: o, solid_r: 0.0, inside, info: Value::Null, warnings: Vec::new() }
        }
        Type::Bevel | Type::Miter => {
            let (delta, ae) = sp.cone();
            let r = sp.m * z / 2.0;
            let zv = z / delta.cos();
            let rv = sp.m * zv / 2.0;
            let t = gp::generate(zv, &k, rv + (k.ha + k.x) * k.m)?;
            tooth_warn(&t, &mut warnings);
            if sp.face > ae / 3.0 + 1e-9 {
                warnings.push(format!("the face width is more than a third of the cone distance ({:.2} mm)", ae / 3.0));
            }
            if sp.face >= 0.8 * ae {
                return Err(format!("the face width must be well under the cone distance ({ae:.2} mm)"));
            }
            info.insert("pitch_diameter".into(), mm(2.0 * r));
            info.insert("pitch_cone_angle".into(), dg(delta));
            info.insert("cone_distance".into(), mm(ae));
            info.insert("virtual_teeth".into(), mm(zv));
            info.insert("ratio".into(), mm(sp.mate_teeth as f64 / z));
            let theta_a = (sp.m * (sp.ha + sp.x) / ae).atan();
            let theta_f = (sp.m * (sp.ha + sp.c - sp.x) / ae).atan();
            info.insert("face_angle".into(), dg(delta + theta_a));
            info.insert("root_angle".into(), dg(delta - theta_f));
            info.insert("tip_diameter".into(), mm(2.0 * (r + sp.m * (sp.ha + sp.x) * delta.cos())));
            if sp.helix > 0.0 {
                info.insert("spiral_angle".into(), dg(sp.helix));
                warnings.push("spiral teeth are approximate (a twisted taper), for looks and light duty".into());
            }
            let o = gp::bevel_outline(&t, sp.teeth, delta, r);
            // The root circle in the heel plane.
            let solid_r = t.rf * delta.cos() * (r / delta.tan()) / (r / delta.tan() - (-r * delta.tan() + t.rf * delta.sin()));
            Plan { outline: o, solid_r, inside: Vec2::ZERO, info: Value::Null, warnings: Vec::new() }
        }
        Type::Worm => {
            let (lead, gamma) = sp.lead();
            let r1 = sp.worm_d / 2.0;
            let (o, ra, rf) = gp::worm_section(&k, r1, sp.starts, lead)?;
            info.insert("pitch_diameter".into(), mm(sp.worm_d));
            info.insert("tip_diameter".into(), mm(2.0 * ra));
            info.insert("root_diameter".into(), mm(2.0 * rf));
            info.insert("axial_pitch".into(), mm(PI * sp.m));
            info.insert("lead".into(), mm(lead));
            info.insert("lead_angle".into(), dg(gamma));
            info.insert("turns".into(), mm(sp.face / lead));
            info.insert("length".into(), mm(sp.face));
            if gamma > 25f64.to_radians() {
                warnings.push("a lead angle over 25° is steep for a worm: use a larger worm diameter or fewer starts".into());
            }
            Plan { outline: o, solid_r: rf, inside: Vec2::ZERO, info: Value::Null, warnings: Vec::new() }
        }
    };
    if sp.bore > 0.0 && sp.ty.round() {
        let wall = out.solid_r - sp.bore / 2.0;
        if sp.ty == Type::Internal {
            warnings.push("an internal gear has no bore".into());
        } else if wall < 0.5 * sp.m.min(1.0) {
            return Err(format!("the bore (Ø{}) is too large for the root diameter (Ø{:.2})", trim(sp.bore), 2.0 * out.solid_r));
        }
        if sp.keyway && sp.ty != Type::Internal {
            match key_for(sp.bore) {
                Some((b, t2)) => {
                    info.insert("keyway".into(), json!(format!("{} x {} (DIN 6885)", trim(b), trim(t2))));
                    if sp.bore / 2.0 + t2 > out.solid_r - 0.5 * sp.m.min(1.0) {
                        return Err("the keyway would cut through the root".into());
                    }
                }
                None => warnings.push("DIN 6885 has no key for this bore (6–130 mm): no keyway".into()),
            }
        }
    }
    let mut out = out;
    if sp.ty.round() && sp.ty != Type::Internal {
        // Between the bore and the root, away from the keyway (towards +x).
        out.inside = Vec2::new(-(sp.bore.max(0.0) / 2.0 + out.solid_r) / 2.0, 0.0);
    }
    out.info = Value::Object(info);
    out.warnings = warnings;
    Ok(out)
}

fn rotate(g: &Geom2, a: f64) -> Geom2 {
    let (s, c) = a.sin_cos();
    let r = |p: &Vec2| Vec2::new(p.x * c - p.y * s, p.x * s + p.y * c);
    match g {
        Geom2::Line(a, b) => Geom2::Line(r(a), r(b)),
        Geom2::Arc { c: cc, a, b } => Geom2::Arc { c: r(cc), a: r(a), b: r(b) },
        Geom2::Spline { pts, control, degree } => Geom2::Spline { pts: pts.iter().map(r).collect(), control: *control, degree: *degree },
        Geom2::Circle(cc, rr) => Geom2::Circle(r(cc), *rr),
        Geom2::Point(q) => Geom2::Point(r(q)),
        o => o.clone(),
    }
}

fn run(s: &mut Session, id: &str, p: Value) -> Result<Value> {
    s.execute(id, &p)
}

/// A sketch on the plane z = `z0` holding `geom`; its id.
fn profile(s: &mut Session, geom: &[Geom2], z0: f64, name: &str) -> Result<u64> {
    run(s, "sketch.create", json!({"plane": "XY", "offset": format!("{z0} mm"), "name": name, "project_edges": false}))?;
    let id = s.active_sketch.ok_or_else(|| EngineError::Other("gear: the profile sketch was not created".into()))?;
    super::sketch_import::add_to_active(s, geom, "gear.create")?;
    run(s, "sketch.finish", json!({}))?;
    // Evaluate the sketch's profiles, so the feature that uses it can pick one by a point.
    s.refresh();
    Ok(id)
}

/// A sketch with one line from `a` to `b` (world coordinates on the XZ plane, y = 0): a
/// sweep's path.
fn path(s: &mut Session, a: (f64, f64), b: (f64, f64), name: &str) -> Result<u64> {
    run(s, "sketch.create", json!({"plane": "XZ", "name": name, "project_edges": false}))?;
    let id = s.active_sketch.ok_or_else(|| EngineError::Other("gear: the path sketch was not created".into()))?;
    // The XZ sketch's y runs down world z.
    run(s, "sketch.line", json!({"points": [[a.0, -a.1], [b.0, -b.1]], "ids": ["axis"]}))?;
    run(s, "sketch.finish", json!({}))?;
    Ok(id)
}

/// The profile around `at` (with its holes).
fn pick(at: Vec2) -> Value {
    json!([{"point": [at.x, at.y]}])
}

#[allow(clippy::too_many_arguments)]
fn sweep(s: &mut Session, prof: u64, at: Vec2, path_id: u64, twist: f64, scale: Option<f64>, parallel: bool, op: &str, name: &str) -> Result<()> {
    let mut p = json!({"sketch": prof, "profiles": pick(at), "path_sketch": path_id, "path": ["axis"], "operation": op, "body_name": name});
    if twist.abs() > 1e-12 {
        p["twist"] = json!(format!("{} deg", twist.to_degrees()));
    }
    if let Some(k) = scale {
        p["scale"] = json!(k);
    }
    if parallel {
        p["orientation"] = json!("parallel");
    }
    run(s, "solid.sweep", p)?;
    Ok(())
}

/// Most twist one sweep takes (just inside the kernel's four turns).
const MAX_SWEEP_TWIST: f64 = 3.99 * TAU;

/// The bore as a hole in the profile: a circle, or (on teeth that neither twist nor taper, so the
/// keyway runs straight) a circle with the DIN 6885 keyway's notch towards +x.
fn bore_loop(sp: &Spec, keyway: bool) -> Vec<Geom2> {
    let r = sp.bore / 2.0;
    match key_for(sp.bore).filter(|_| keyway) {
        Some((b, t2)) if b / 2.0 < r => {
            let th = (b / 2.0 / r).asin();
            let (c, s) = (r * th.cos(), r * th.sin());
            let x = r + t2;
            vec![
                Geom2::Arc { c: Vec2::ZERO, a: Vec2::new(c, s), b: Vec2::new(c, -s) },
                Geom2::Line(Vec2::new(c, -s), Vec2::new(x, -s)),
                Geom2::Line(Vec2::new(x, -s), Vec2::new(x, s)),
                Geom2::Line(Vec2::new(x, s), Vec2::new(c, s)),
            ]
        }
        _ => vec![Geom2::Circle(Vec2::ZERO, r)],
    }
}

/// Build the gear's body in the active component.
fn build(s: &mut Session, sp: &Spec) -> Result<Plan> {
    let mut pl = plan(sp).map_err(|e| bad(CMD, e))?;
    // The bore goes into the profile (no boolean against the teeth), except on bevel teeth,
    // which taper (it would too). A keyway twisting with helical teeth would be wrong: those
    // get theirs cut afterwards.
    let straight = sp.twist() == 0.0 && !matches!(sp.ty, Type::Bevel | Type::Miter);
    let hub = sp.hub_d > 0.0 && sp.hub_len > 0.0;
    // A hub joined to swept teeth must reach into them (a join on their flat face alone is very
    // slow), so the bore is then cut through both afterwards.
    let bore_in_profile = sp.bore > 0.0 && sp.ty.round() && !matches!(sp.ty, Type::Internal | Type::Bevel | Type::Miter) && (straight || !hub);
    if bore_in_profile {
        pl.outline.extend(bore_loop(sp, sp.keyway && straight));
    }
    let _frame = crate::frames::local_geometry();
    // Body names are the design's: a second gear's body can't take the first one's name, and
    // the steps below find this one's body by its name.
    let body_name = unique_body(s, &sp.name());
    let body = body_name.as_str();
    let f = sp.face;
    match sp.ty {
        Type::Spur => {
            let prof = profile(s, &pl.outline, 0.0, "Tooth Profile")?;
            run(
                s,
                "solid.extrude",
                json!({"sketch": prof, "profiles": pick(pl.inside), "distance": format!("{f} mm"), "operation": "new", "body_name": body}),
            )?;
        }
        Type::Rack => {
            let prof = profile(s, &pl.outline, 0.0, "Tooth Profile")?;
            if sp.helix == 0.0 {
                run(
                    s,
                    "solid.extrude",
                    json!({"sketch": prof, "profiles": pick(pl.inside), "distance": format!("{f} mm"), "operation": "new", "body_name": body}),
                )?;
            } else {
                // The teeth lean along +x going up (right hand) or −x (left hand).
                let dx = f * sp.helix.tan() * if sp.left { -1.0 } else { 1.0 };
                let pa = path(s, (0.0, 0.0), (dx, f), "Helix")?;
                sweep(s, prof, pl.inside, pa, 0.0, None, true, "new", body)?;
            }
        }
        Type::Bevel | Type::Miter => {
            let prof = profile(s, &pl.outline, 0.0, "Tooth Profile")?;
            let (delta, ae) = sp.cone();
            let apex = sp.pitch_radius() / delta.tan();
            let len = apex * f / ae;
            let pa = path(s, (0.0, 0.0), (0.0, len), "Cone Axis")?;
            sweep(s, prof, pl.inside, pa, sp.twist(), Some((ae - f) / ae), false, "new", body)?;
        }
        Type::Herringbone => {
            let prof = profile(s, &pl.outline, 0.0, "Tooth Profile")?;
            let pa = path(s, (0.0, 0.0), (0.0, f / 2.0), "Helix")?;
            sweep(s, prof, pl.inside, pa, sp.twist(), None, false, "new", body)?;
            run(s, "solid.mirror", json!({"bodies": [body], "plane": {"origin": [0, 0, f / 2.0], "normal": [0, 0, 1]}, "combine": true}))?;
        }
        Type::Helical | Type::Crossed | Type::Internal | Type::WormWheel if sp.twist() == 0.0 => {
            let prof = profile(s, &pl.outline, 0.0, "Tooth Profile")?;
            run(
                s,
                "solid.extrude",
                json!({"sketch": prof, "profiles": pick(pl.inside), "distance": format!("{f} mm"), "operation": "new", "body_name": body}),
            )?;
        }
        Type::Helical | Type::Crossed | Type::Internal | Type::WormWheel | Type::Worm => {
            // Over four turns (long worms), the sweep goes in pieces, each turned on.
            let total = sp.twist();
            let pieces = (total.abs() / MAX_SWEEP_TWIST).ceil().max(1.0) as usize;
            for i in 0..pieces {
                let (z0, z1) = (f * i as f64 / pieces as f64, f * (i + 1) as f64 / pieces as f64);
                let turn = total * i as f64 / pieces as f64;
                let turned: Vec<Geom2> = pl.outline.iter().map(|g| rotate(g, turn)).collect();
                let at = match rotate(&Geom2::Point(pl.inside), turn) {
                    Geom2::Point(q) => q,
                    _ => pl.inside,
                };
                let prof = profile(s, &turned, z0, if i == 0 { "Tooth Profile".to_string() } else { format!("Tooth Profile {}", i + 1) }.as_str())?;
                let pa = path(s, (0.0, z0), (0.0, z1), if i == 0 { "Helix".to_string() } else { format!("Helix {}", i + 1) }.as_str())?;
                sweep(s, prof, at, pa, total / pieces as f64, None, false, if i == 0 { "new" } else { "join" }, body)?;
            }
        }
    }
    if sp.ty.round() && sp.ty != Type::Internal {
        if hub && straight {
            // A ring below z = 0, its hole the bore (with the keyway where the teeth have theirs).
            let mut ring = vec![Geom2::Circle(Vec2::ZERO, sp.hub_d / 2.0)];
            if bore_in_profile {
                ring.extend(bore_loop(sp, sp.keyway));
            }
            let hub = profile(s, &ring, 0.0, "Hub")?;
            run(
                s,
                "solid.extrude",
                json!({"sketch": hub, "profiles": pick(Vec2::new(-(sp.bore / 2.0 + sp.hub_d / 2.0) / 2.0, 0.0)), "distance": format!("{} mm", sp.hub_len), "direction": "negative", "operation": "join", "targets": [body]}),
            )?;
        } else if hub {
            // Into the swept teeth by a little (within their solid core).
            let into = (0.25 * sp.face).min(sp.m);
            run(
                s,
                "solid.cylinder",
                json!({"diameter": format!("{} mm", sp.hub_d.min(2.0 * pl.solid_r - sp.m)), "height": format!("{} mm", sp.hub_len + into), "base": [0, 0, -sp.hub_len], "axis": [0, 0, 1], "operation": "join", "targets": [body]}),
            )?;
        }
        let (z0, z1) = (-sp.hub_len - 1.0, axial_top(sp) + 1.0);
        if sp.bore > 0.0 && !bore_in_profile {
            // Through tapering bevel teeth, or teeth and hub together.
            run(
                s,
                "solid.cylinder",
                json!({"diameter": format!("{} mm", sp.bore), "height": format!("{} mm", z1 - z0), "base": [0, 0, z0], "axis": [0, 0, 1], "operation": "cut", "targets": [body]}),
            )?;
        }
        if sp.bore > 0.0
            && sp.keyway
            && !(straight && bore_in_profile)
            && let Some((b, t2)) = key_for(sp.bore)
        {
            run(
                s,
                "solid.box",
                json!({"length": format!("{} mm", sp.bore / 2.0 + t2), "width": format!("{b} mm"), "height": format!("{} mm", z1 - z0), "corner": [0, -b / 2.0, z0], "operation": "cut", "targets": [body]}),
            )?;
        }
    }
    Ok(pl)
}

/// `base`, or `base 2`, `base 3`… : a body name no body in the design has (or had).
fn unique_body(s: &Session, base: &str) -> String {
    let st = s.world_state();
    let taken = |n: &str| st.bodies.iter().any(|b| b.name == n) || s.doc.reserved_body_names.contains(n);
    if !taken(base) {
        return base.to_string();
    }
    (2..10_000).map(|i| format!("{base} {i}")).find(|n| !taken(n)).unwrap_or_else(|| format!("{base} {}", s.doc.next_id))
}

/// Where the gear's body ends up the axis.
fn axial_top(sp: &Spec) -> f64 {
    match sp.ty {
        Type::Bevel | Type::Miter => {
            let (delta, ae) = sp.cone();
            sp.pitch_radius() / delta.tan() * sp.face / ae
        }
        _ => sp.face,
    }
}

fn info(s: &mut Session, p: &Value) -> Result<Value> {
    let sp = Spec::parse(&s.doc, p)?;
    let pl = plan(&sp).map_err(|e| bad("gear.info", e))?;
    let mut out = pl.info;
    out["warnings"] = json!(pl.warnings);
    out["parameters"] = sp.to_json();
    out["types"] = json!(TYPES.iter().map(|t| json!({"type": t.0, "name": t.1})).collect::<Vec<_>>());
    // With a mate: the pair's centre distance and contact ratio (parallel axes).
    if let Some(z2) = count(&s.doc, p, "mate_teeth", 3, 1000)?
        && matches!(sp.ty, Type::Spur | Type::Helical | Type::Herringbone)
    {
        let k = sp.rack();
        let (z1, z2) = (sp.teeth as f64, z2 as f64);
        let x2 = num(p, "mate_profile_shift").unwrap_or(0.0);
        let xs = (sp.x + x2) * sp.m / k.m;
        // Working pressure angle from inv αw = inv α + 2 tan α (x1 + x2) / (z1 + z2).
        let target = gp::inv(k.alpha) + 2.0 * k.alpha.tan() * xs / (z1 + z2);
        let mut aw = k.alpha;
        for _ in 0..60 {
            let f = gp::inv(aw) - target;
            let d = aw.tan().powi(2);
            if d.abs() < 1e-15 {
                break;
            }
            aw = (aw - f / d).clamp(1e-3, 1.5);
        }
        let a0 = k.m * (z1 + z2) / 2.0;
        let a = a0 * k.alpha.cos() / aw.cos();
        let (r1, r2) = (k.m * z1 / 2.0, k.m * z2 / 2.0);
        let (rb1, rb2) = (r1 * k.alpha.cos(), r2 * k.alpha.cos());
        let (ra1, ra2) = (r1 + (k.ha + k.x) * k.m, r2 + (k.ha + x2 * sp.m / k.m) * k.m);
        let eps = ((ra1 * ra1 - rb1 * rb1).sqrt() + (ra2 * ra2 - rb2 * rb2).sqrt() - a * aw.sin()) / (PI * k.m * k.alpha.cos());
        out["centre_distance"] = json!((a * 1e6).round() / 1e6);
        out["working_pressure_angle"] = json!((aw.to_degrees() * 1e6).round() / 1e6);
        out["contact_ratio"] = json!((eps * 1e4).round() / 1e4);
        out["ratio"] = json!((z2 / z1 * 1e6).round() / 1e6);
        if sp.ty.helical() {
            let overlap = sp.face * sp.helix.sin() / (PI * sp.m);
            out["overlap_ratio"] = json!((overlap * 1e4).round() / 1e4);
        }
    }
    Ok(out)
}

/// The last gear built: from which design (by identity), with which command and parameters, and
/// the session it came to. The dialog's preview and OK's evaluation run the command off the
/// window thread first; OK then runs it again on the window thread, which takes this result
/// instead of building it all again (many evaluations and booleans: seconds on a tablet, long
/// enough for Android to call the app unresponsive).
struct Built {
    doc: std::sync::Arc<solidcraft_doc::Document>,
    revision: u64,
    cmd: &'static str,
    params: Value,
    computed: Session,
    result: Value,
}

static LAST: std::sync::Mutex<Option<Built>> = std::sync::Mutex::new(None);

/// Build on a scratch session (one undo step), or take the same build done just before.
fn built(s: &mut Session, p: &Value, cmd: &'static str, f: fn(&mut Session, &Value) -> Result<Value>) -> Result<(Session, Value)> {
    let same = |b: &Built| b.cmd == cmd && std::sync::Arc::ptr_eq(&b.doc, &s.doc) && b.revision == s.revision && b.params == *p;
    // Kept, not taken: the preview, OK's evaluation ahead and OK's run all build the same.
    if let Ok(last) = LAST.lock()
        && let Some(b) = last.as_ref().filter(|b| same(b))
    {
        return Ok((b.computed.scratch(), b.result.clone()));
    }
    let (doc, revision) = (s.doc.clone(), s.revision);
    let mut computed = s.scratch();
    let result = f(&mut computed, p)?;
    computed.undo.clear();
    if let Ok(mut last) = LAST.lock() {
        *last = Some(Built { doc, revision, cmd, params: p.clone(), computed: computed.scratch(), result: result.clone() });
    }
    Ok((computed, result))
}

fn create_atomic(s: &mut Session, p: &Value) -> Result<Value> {
    let (computed, result) = built(s, p, "gear.create", create)?;
    s.accept_computed(computed);
    Ok(result)
}

fn edit_atomic(s: &mut Session, p: &Value) -> Result<Value> {
    let (mut computed, result) = built(s, p, "gear.edit", edit)?;
    computed.active_component = s.active_component;
    computed.active_occurrence_path = s.active_occurrence_path.clone();
    computed.active_sketch = s.active_sketch.filter(|id| computed.doc.feature(*id).is_some());
    s.accept_computed(computed);
    Ok(result)
}

fn store(s: &mut Session, comp: u64, sp: &Spec) {
    use solidcraft_doc::metadata::{DESCRIPTION, Owner, PART_NUMBER};
    let Some(c) = s.doc.components.iter().find(|c| c.id == comp) else { return };
    let mut m = c.metadata.clone();
    let _ = m.set(Owner::Component, KEY, sp.to_json());
    let _ = m.set(Owner::Component, PART_NUMBER, Value::String(sp.name()));
    let _ = m.set(Owner::Component, DESCRIPTION, Value::String(format!("{}, module {} mm, {} teeth", sp.ty.label(), trim(sp.m), sp.teeth)));
    if let Some(c) = s.doc_mut().components.iter_mut().find(|c| c.id == comp) {
        c.metadata = m;
    }
}

/// A gear component's stored parameters.
pub fn stored(s: &Session, comp: u64) -> Option<Value> {
    s.doc.components.iter().find(|c| c.id == comp).and_then(|c| c.metadata.0.get(KEY).cloned())
}

fn check_built(s: &mut Session) -> Result<()> {
    s.refresh();
    if let Some(e) = s.model.results.iter().find_map(|r| r.error.clone()) {
        return Err(EngineError::Other(format!("the gear could not be built: {e}")));
    }
    Ok(())
}

fn create(s: &mut Session, p: &Value) -> Result<Value> {
    let sp = Spec::parse(&s.doc, p)?;
    // Refuse before anything is made.
    plan(&sp).map_err(|e| bad(CMD, e))?;
    let name = str_(p, "name").map(str::trim).filter(|n| !n.is_empty()).map(str::to_string).unwrap_or_else(|| sp.name());
    let active = s.active_component;
    let c = run(s, "component.create", json!({"name": name, "grounded": false}))?;
    let (comp, occ) = (c["component"].as_u64().unwrap_or(0), c["occurrence"].as_u64().unwrap_or(0));
    let pl = build(s, &sp)?;
    store(s, comp, &sp);
    run(s, "component.activate", json!({"component": active}))?;
    if let Some(pt) = p.get("point").and_then(vec3) {
        let dir = p.get("direction").and_then(vec3).and_then(|d| d.normalized()).unwrap_or(Vec3::Z);
        let mut m = solidcraft_doc::rigid(
            Vec3::ZERO,
            Vec3::ZERO,
            Vec3::Z.cross(dir).normalized().unwrap_or(Vec3::X),
            Vec3::Z.dot(dir).clamp(-1.0, 1.0).acos(),
        );
        m[3] = [pt.x, pt.y, pt.z, 1.0];
        if let Some(o) = s.doc_mut().occurrences.iter_mut().find(|o| o.id == occ) {
            o.transform = m;
        }
    }
    check_built(s)?;
    Ok(json!({"component": comp, "occurrence": occ, "name": name, "info": pl.info, "warnings": pl.warnings}))
}

fn edit(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "gear.edit";
    let key = match p.get("component") {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(x)) => x.clone(),
        _ => return Err(bad(cmd, "`component` must be an id or a name")),
    };
    let comp = s.doc.find_component(&key).ok_or_else(|| bad(cmd, format!("no component `{key}`")))?;
    let mut merged = stored(s, comp).filter(Value::is_object).ok_or_else(|| bad(cmd, "that component is not a gear"))?;
    if let (Value::Object(m), Value::Object(new)) = (&mut merged, p) {
        for (k, v) in new.iter().filter(|(k, _)| !matches!(k.as_str(), "component" | "name" | "point" | "direction")) {
            m.insert(k.clone(), v.clone());
        }
        // A new type starts from that type's defaults for what only it has.
        if new.contains_key("type") {
            for k in ["face_width", "helix_angle", "teeth"] {
                if !new.contains_key(k) {
                    m.remove(k);
                }
            }
        }
    }
    let sp = Spec::parse(&s.doc, &merged)?;
    plan(&sp).map_err(|e| bad(cmd, e))?;
    let old_name = s.doc.components.iter().find(|c| c.id == comp).map(|c| c.name.clone()).unwrap_or_default();
    let was_auto = stored(s, comp).and_then(|v| Spec::parse(&s.doc, &v).ok()).is_some_and(|o| o.name() == old_name);
    let ids: Vec<u64> = s.doc.features.iter().filter(|f| f.component == comp).map(|f| f.id).collect();
    for id in ids.iter().rev() {
        let _ = s.doc_mut().delete_feature(*id);
    }
    // Its old body is gone: its name is free again.
    s.refresh();
    let active = s.active_component;
    run(s, "component.activate", json!({"component": comp}))?;
    let pl = build(s, &sp)?;
    store(s, comp, &sp);
    let name = match str_(p, "name").map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => n.to_string(),
        None if was_auto => sp.name(),
        None => old_name.clone(),
    };
    if let Some(c) = s.doc_mut().components.iter_mut().find(|c| c.id == comp) {
        c.name = name.clone();
    }
    // Its placements named after it (`Spur Gear m1 z20:1`) follow the new name.
    if name != old_name {
        let prefix = format!("{old_name}:");
        for o in s.doc_mut().occurrences.iter_mut().filter(|o| o.component == comp) {
            if let Some(n) = o.name.strip_prefix(&prefix).filter(|n| n.chars().all(|c| c.is_ascii_digit())) {
                o.name = format!("{name}:{n}");
            }
        }
    }
    run(s, "component.activate", json!({"component": active}))?;
    check_built(s)?;
    Ok(json!({"component": comp, "name": name, "info": pl.info, "warnings": pl.warnings}))
}

#[cfg(test)]
#[path = "gears_tests.rs"]
mod tests;
