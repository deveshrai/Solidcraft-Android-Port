use serde_json::{Value, json};
use solidcraft_geom::{Mesh, Vec3};
use std::f64::consts::PI;

use crate::Session;

fn run(s: &mut Session, id: &str, p: Value) -> Value {
    match s.execute(id, &p) {
        Ok(v) => v,
        Err(e) => panic!("{id} {p}: {e}"),
    }
}

/// The meshes of a component's bodies, in world coordinates.
fn meshes(s: &Session, comp: u64) -> Vec<Mesh> {
    let st = s.world_state();
    st.bodies.iter().filter(|b| s.doc.model_body_component(b) == comp).map(|b| b.mesh().as_ref().clone()).collect()
}

fn gear(s: &mut Session, p: Value) -> (u64, Vec<Mesh>) {
    let r = run(s, "gear.create", p);
    let comp = r["component"].as_u64().unwrap_or(0);
    let m = meshes(s, comp);
    assert_eq!(m.len(), 1, "one body: {r}");
    (comp, m)
}

fn bounds(m: &[Mesh]) -> (Vec3, Vec3) {
    let b = m.iter().fold(solidcraft_geom::Aabb3::EMPTY, |a, m| a.union(&m.bounds()));
    (b.min, b.max)
}

fn volume(m: &[Mesh]) -> f64 {
    m.iter().map(|m| m.measure().volume).sum()
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// A spur gear: tip, root and face as the standard relations give them, one undo step, and
/// the teeth's volume about what the involute tooth's area says.
#[test]
fn a_spur_gear_has_its_standard_dimensions() {
    let mut s = Session::default();
    let undo = s.undo.len();
    let (comp, m) = gear(&mut s, json!({"type": "spur", "module": 2, "teeth": 20, "face_width": 10}));
    assert_eq!(s.undo.len(), undo + 1);
    let (lo, hi) = bounds(&m);
    // Tooth 0 on +x and (20 teeth) tooth 10 on −x: the tip diameter is m (z + 2) = 44 (the
    // display mesh's chords sit a little inside the tip arcs).
    assert!(close(hi.x, 22.0, 0.02) && close(lo.x, -22.0, 0.02), "{lo:?} {hi:?}");
    let far = m.iter().flat_map(|m| m.positions.iter()).map(|p| p.x.hypot(p.y)).fold(0.0, f64::max);
    assert!(far <= 22.0 + 1e-6, "nothing outside the tip circle: {far}");
    assert!(close(lo.z, 0.0, 1e-6) && close(hi.z, 10.0, 1e-6));
    // The tooth's area is close to half the annulus between root and tip, plus the root disc.
    let v = volume(&m);
    let (ra, rf) = (22.0, 17.5);
    let rough = PI * rf * rf * 10.0 + 0.5 * PI * (ra * ra - rf * rf) * 10.0;
    assert!(v > 0.9 * rough && v < 1.1 * rough, "{v} vs {rough}");
    let stored = super::stored(&s, comp).unwrap_or_default();
    assert_eq!(stored["type"], "spur");
    assert_eq!(stored["teeth"], 20);
}

/// The helix turns the right way: a right-hand gear's teeth turn counter-clockwise going up.
#[test]
fn helical_teeth_turn_with_their_hand() {
    for (hand, sign) in [("right", 1.0), ("left", -1.0)] {
        let mut s = Session::default();
        let r = run(&mut s, "gear.info", json!({"type": "helical", "module": 2, "teeth": 20, "helix_angle": 20, "face_width": 8, "hand": hand}));
        let twist = r["twist"].as_f64().unwrap_or(0.0).to_radians();
        assert!(close(twist, sign * 8.0 * 20f64.to_radians().tan() / (20.0 / 20f64.to_radians().cos()), 1e-6), "{r}");
        let (_, m) = gear(&mut s, json!({"type": "helical", "module": 2, "teeth": 20, "helix_angle": 20, "face_width": 8, "hand": hand}));
        // The tips at the top face sit around the teeth's centres, turned by the twist.
        let ra = r["tip_diameter"].as_f64().unwrap_or(0.0) / 2.0;
        let pitch = 2.0 * PI / 20.0;
        let (mut sum, mut n) = (0.0, 0.0);
        for p in m.iter().flat_map(|m| m.positions.iter()) {
            if close(p.z, 8.0, 1e-6) && p.x.hypot(p.y) > ra - 0.05 {
                let a = p.y.atan2(p.x) - twist;
                sum += a - pitch * (a / pitch).round();
                n += 1.0;
            }
        }
        assert!(n > 10.0 && (sum / n).abs() < 0.01, "{hand}: tips off the turned centres by {}", sum / n);
        // A helical gear has the same transverse section all the way up.
        let v = volume(&m);
        let mut flat = Session::default();
        let (_, fm) = gear(
            &mut flat,
            json!({"type": "spur", "module": 2.0 / 20f64.to_radians().cos(), "pressure_angle": (20f64.to_radians().tan() / 20f64.to_radians().cos()).atan().to_degrees(), "teeth": 20, "face_width": 8, "addendum": 20f64.to_radians().cos(), "clearance": 0.25 * 20f64.to_radians().cos(), "root_fillet": 0.38 * 20f64.to_radians().cos()}),
        );
        assert!(close(v, volume(&fm), 0.01 * v), "{v} vs {}", volume(&fm));
    }
}

/// Every type builds with its defaults.
#[test]
fn every_type_builds() {
    for (ty, extra) in [
        ("spur", json!({})),
        ("helical", json!({})),
        ("herringbone", json!({"face_width": 12})),
        ("internal", json!({"teeth": 48})),
        ("rack", json!({"teeth": 8})),
        ("rack", json!({"teeth": 8, "helix_angle": 15})),
        ("bevel", json!({"teeth": 20, "mate_teeth": 40})),
        ("bevel", json!({"teeth": 20, "mate_teeth": 30, "helix_angle": 30})),
        ("miter", json!({"teeth": 16})),
        ("worm", json!({"starts": 2})),
        ("worm_wheel", json!({"teeth": 30})),
        ("crossed_helical", json!({"teeth": 15})),
    ] {
        let mut s = Session::default();
        let mut p = json!({"type": ty, "module": 1.5});
        if let (Value::Object(o), Value::Object(e)) = (&mut p, extra) {
            o.extend(e);
        }
        let (_, m) = gear(&mut s, p.clone());
        let (lo, hi) = bounds(&m);
        assert!(hi.x > lo.x && hi.y > lo.y && hi.z > lo.z && volume(&m) > 0.0, "{p}: {lo:?} {hi:?}");
    }
}

/// A bevel gear tapers towards its pitch apex: its toe is the heel scaled by (Ae − F) / Ae.
#[test]
fn bevel_teeth_taper_to_the_apex() {
    let mut s = Session::default();
    let i = run(&mut s, "gear.info", json!({"type": "miter", "module": 2, "teeth": 20, "face_width": 8}));
    assert!(close(i["pitch_cone_angle"].as_f64().unwrap_or(0.0), 45.0, 1e-9), "{i}");
    let ae = 20.0 / 45f64.to_radians().sin();
    assert!(close(i["cone_distance"].as_f64().unwrap_or(0.0), ae, 1e-6));
    let (_, m) = gear(&mut s, json!({"type": "miter", "module": 2, "teeth": 20, "face_width": 8}));
    let (lo, hi) = bounds(&m);
    // The heel is at z = 0; the face runs 8 mm along the cone, 8 cos 45° up the axis.
    assert!(close(lo.z, 0.0, 1e-6) && close(hi.z, 8.0 * 45f64.to_radians().cos(), 1e-3), "{lo:?} {hi:?}");
    let heel = m.iter().flat_map(|m| m.positions.iter()).filter(|p| p.z.abs() < 1e-6).map(|p| p.x.hypot(p.y)).fold(0.0, f64::max);
    let toe = m.iter().flat_map(|m| m.positions.iter()).filter(|p| (p.z - hi.z).abs() < 1e-6).map(|p| p.x.hypot(p.y)).fold(0.0, f64::max);
    assert!(close(toe / heel, (ae - 8.0) / ae, 2e-3), "{toe} / {heel}");
}

/// A worm's length can be over the four turns one sweep makes; its tip is m (q + 2).
#[test]
fn a_long_worm_is_built_in_pieces() {
    let mut s = Session::default();
    let (comp, m) = gear(&mut s, json!({"type": "worm", "module": 1, "starts": 1, "face_width": 20, "worm_diameter": 10}));
    let (lo, hi) = bounds(&m);
    assert!(close(hi.z - lo.z, 20.0, 1e-6) && close(hi.x.max(hi.y), 6.0, 2e-3), "{lo:?} {hi:?}");
    let sweeps = s.doc.features.iter().filter(|f| f.component == comp && matches!(f.kind, solidcraft_doc::FeatureKind::Sweep { .. })).count();
    assert_eq!(sweeps, 2, "20 mm is 6.4 turns of a 3.14 mm lead");
}

/// A worm wheel: its worm's centre distance and lead angle, its teeth turned at that angle.
#[test]
fn a_worm_wheel_follows_its_worm() {
    let mut s = Session::default();
    let i = run(&mut s, "gear.info", json!({"type": "worm_wheel", "module": 2, "teeth": 30, "starts": 1, "worm_diameter": 20, "face_width": 12}));
    assert!(close(i["centre_distance"].as_f64().unwrap_or(0.0), 40.0, 1e-9), "{i}");
    let gamma = (2.0f64 * PI / (PI * 20.0)).atan();
    assert!(close(i["lead_angle"].as_f64().unwrap_or(0.0), gamma.to_degrees(), 1e-6));
    assert!(close(i["twist"].as_f64().unwrap_or(0.0).to_radians(), 12.0 * gamma.tan() / 30.0, 1e-6), "{i}");
    let (_, m) = gear(&mut s, json!({"type": "worm_wheel", "module": 2, "teeth": 30, "worm_diameter": 20, "face_width": 12}));
    let (lo, hi) = bounds(&m);
    assert!(close(hi.z - lo.z, 12.0, 1e-6) && close(hi.x, 32.0, 0.03), "{lo:?} {hi:?}");
}

/// A bore, keyway and hub on swept teeth: no slow boolean against the helices.
#[test]
fn swept_gears_take_a_bore_quickly() {
    for ty in ["helical", "bevel", "herringbone", "worm_wheel"] {
        let mut s = Session::default();
        let t = std::time::Instant::now();
        let r = run(
            &mut s,
            "gear.create",
            json!({"type": ty, "module": 2, "teeth": 24, "bore": 12, "keyway": true, "hub_diameter": 22, "hub_length": 6}),
        );
        let secs = t.elapsed().as_secs_f64();
        let m = meshes(&s, r["component"].as_u64().unwrap_or(0));
        assert_eq!(m.len(), 1, "{ty}");
        let (lo, _) = bounds(&m);
        assert!(close(lo.z, -6.0, 1e-6), "{ty}: the hub reaches down 6 mm: {lo:?}");
        let inside = m.iter().flat_map(|m| m.positions.iter()).map(|p| p.x.hypot(p.y)).fold(f64::MAX, f64::min);
        assert!(close(inside, 6.0, 0.02), "{ty}: the bore is Ø12: {inside}");
        // Generous: the suite runs the heavy tests side by side (alone it is about 9 s).
        assert!(secs < 60.0, "{ty} took {secs:.1} s");
        eprintln!("{ty} with bore, keyway and hub: {secs:.2} s");
    }
}

/// A bore with its DIN 6885 keyway and a hub.
#[test]
fn bore_keyway_and_hub() {
    let mut s = Session::default();
    let (_, plain) = gear(&mut s, json!({"type": "spur", "module": 2, "teeth": 30, "face_width": 10}));
    let mut b = Session::default();
    let r = run(
        &mut b,
        "gear.create",
        json!({"type": "spur", "module": 2, "teeth": 30, "face_width": 10, "bore": 20, "keyway": true, "hub_diameter": 35, "hub_length": 8}),
    );
    assert_eq!(r["info"]["keyway"], "6 x 2.8 (DIN 6885)");
    let m = meshes(&b, r["component"].as_u64().unwrap_or(0));
    let (lo, _) = bounds(&m);
    assert!(close(lo.z, -8.0, 1e-6));
    let hub = PI * (17.5f64 * 17.5 - 10.0 * 10.0) * 8.0;
    let bore = PI * 100.0 * 10.0 + 6.0 * 2.8 * 18.0;
    let expect = volume(&plain) + hub - bore;
    assert!(close(volume(&m), expect, 0.01 * expect), "{} vs {expect}", volume(&m));
}

/// Edit Gear rebuilds it in one undo step, keeping what wasn't changed.
#[test]
fn editing_a_gear_rebuilds_it() {
    let mut s = Session::default();
    let r = run(&mut s, "gear.create", json!({"type": "spur", "module": 1, "teeth": 20, "face_width": 5, "bore": 5}));
    let comp = r["component"].as_u64().unwrap_or(0);
    let before = s.doc.clone();
    let undo = s.undo.len();
    let e = run(&mut s, "gear.edit", json!({"component": comp, "teeth": 40}));
    assert_eq!(e["name"], "Spur Gear m1 z40");
    let occ = s.doc.occurrences.iter().find(|o| o.component == comp).map(|o| o.name.clone()).unwrap_or_default();
    assert_eq!(occ, "Spur Gear m1 z40:1", "the placement's name follows");
    assert_eq!(s.undo.len(), undo + 1);
    let (lo, hi) = bounds(&meshes(&s, comp));
    assert!(close(hi.x - lo.x, 42.0, 0.04) && close(hi.z, 5.0, 1e-6), "{lo:?} {hi:?}");
    assert_eq!(super::stored(&s, comp).unwrap_or_default()["bore"], 5.0);
    // A bad value changes nothing.
    assert!(s.execute("gear.edit", &json!({"component": comp, "teeth": 2})).is_err());
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(s.doc, before);
    // It survives saving and reopening.
    let text = s.doc.to_json();
    let doc = solidcraft_doc::Document::from_json(&text).unwrap_or_default();
    assert_eq!(doc.components.iter().find(|c| c.id == comp).and_then(|c| c.metadata.0.get(super::KEY)).map(|g| g["teeth"].clone()), Some(json!(20)));
    // An automatic face width follows the size.
    let auto = run(&mut s, "gear.create", json!({"type": "spur", "module": 1, "teeth": 20}));
    let auto = auto["component"].as_u64().unwrap_or(0);
    assert!(super::stored(&s, auto).unwrap_or_default()["face_width"].is_null());
    run(&mut s, "gear.edit", json!({"component": auto, "module": 2}));
    assert!(close(bounds(&meshes(&s, auto)).1.z, 20.0, 1e-6), "10 module of the new size");
}

/// The pair's centre distance and contact ratio, standard and with profile shift.
#[test]
fn pair_readouts() {
    let mut s = Session::default();
    let i = run(&mut s, "gear.info", json!({"type": "spur", "module": 2, "teeth": 20, "mate_teeth": 40}));
    assert!(close(i["centre_distance"].as_f64().unwrap_or(0.0), 60.0, 1e-9), "{i}");
    let eps = i["contact_ratio"].as_f64().unwrap_or(0.0);
    assert!(eps > 1.55 && eps < 1.7, "{eps}");
    // x1 + x2 = 0.5: inv αw = inv 20° + 2 tan 20° · 0.5 / 60.
    let i = run(&mut s, "gear.info", json!({"type": "spur", "module": 2, "teeth": 20, "mate_teeth": 40, "profile_shift": 0.5}));
    let aw = i["working_pressure_angle"].as_f64().unwrap_or(0.0).to_radians();
    let target = crate::gear_profile::inv(20f64.to_radians()) + 2.0 * 20f64.to_radians().tan() * 0.5 / 60.0;
    // The readouts are rounded to 1e-6.
    assert!(close(crate::gear_profile::inv(aw), target, 1e-8));
    assert!(close(i["centre_distance"].as_f64().unwrap_or(0.0), 60.0 * 20f64.to_radians().cos() / aw.cos(), 1e-5));
}

/// Hostile and impossible parameters are refused without building anything.
#[test]
fn bad_gears_are_refused() {
    for p in [
        json!({"type": "sprocket"}),
        json!({"module": -1}),
        json!({"module": f64::MAX}),
        json!({"teeth": 2}),
        json!({"teeth": 20.5}),
        json!({"teeth": 1e12}),
        json!({"pressure_angle": 80}),
        json!({"hand": "both"}),
        json!({"type": "spur", "module": 1, "teeth": 20, "bore": 30}),
        json!({"type": "bevel", "teeth": 20, "face_width": 100}),
        json!({"type": "worm", "starts": 0}),
        json!({"type": "internal", "teeth": 5}),
        json!({"face_width": "lots"}),
    ] {
        let mut s = Session::default();
        let before = s.doc.clone();
        assert!(s.execute("gear.create", &p).is_err(), "{p}");
        assert_eq!(s.doc, before, "{p}");
    }
}

/// Two gears in one design each get their own body, and each one's bore, keyway and hub go to
/// it, not the other.
#[test]
fn two_gears_keep_their_own_bodies() {
    let mut s = Session::default();
    let a = run(&mut s, "gear.create", json!({"type": "spur", "module": 2, "teeth": 20, "face_width": 8}));
    let b = run(
        &mut s,
        "gear.create",
        json!({"type": "spur", "module": 2, "teeth": 20, "face_width": 8, "bore": 10, "keyway": true, "hub_diameter": 20, "hub_length": 5}),
    );
    let (ma, mb) = (meshes(&s, a["component"].as_u64().unwrap_or(0)), meshes(&s, b["component"].as_u64().unwrap_or(0)));
    assert_eq!((ma.len(), mb.len()), (1, 1));
    assert!(close(bounds(&ma).0.z, 0.0, 1e-6), "the first gear has no hub");
    assert!(close(bounds(&mb).0.z, -5.0, 1e-6), "the second gear's hub");
    let inner = |m: &[Mesh]| m.iter().flat_map(|m| m.positions.iter()).map(|p| p.x.hypot(p.y)).fold(f64::MAX, f64::min);
    assert!(inner(&ma) < 4.0 && close(inner(&mb), 5.0, 0.02), "only the second gear is bored: {} {}", inner(&ma), inner(&mb));
    let names: Vec<String> = s.world_state().bodies.iter().map(|b| b.name.clone()).collect();
    assert!(names.contains(&"Spur Gear m2 z20".to_string()) && names.contains(&"Spur Gear m2 z20 2".to_string()), "{names:?}");
}

/// The same gear built again on the same design takes the build just done (the dialog's preview
/// and OK run it off the window thread first): the same result, at once, one undo step.
#[test]
fn building_again_takes_the_last_build() {
    let mut s = Session::default();
    let p = json!({"type": "helical", "module": 2, "teeth": 24, "bore": 10});
    let mut ahead = s.scratch();
    let t = std::time::Instant::now();
    let first = run(&mut ahead, "gear.create", p.clone());
    let cold = t.elapsed();
    let undo = s.undo.len();
    let t = std::time::Instant::now();
    let again = run(&mut s, "gear.create", p);
    let warm = t.elapsed();
    assert_eq!(first, again);
    assert_eq!(s.doc, ahead.doc);
    assert_eq!(s.undo.len(), undo + 1);
    assert!(warm * 5 < cold, "{warm:?} vs {cold:?}");
}
