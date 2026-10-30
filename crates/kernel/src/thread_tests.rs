//! Faceted booleans with helical tools (modelled threads).

use std::f64::consts::PI;

use solidcraft_geom::{Plane, Vec2, Vec3};

use crate::{BoolOp, cylinder, faceted_boolean, sweep_helix_mesh};

/// A trapezoid groove screwed into a shaft: the result is closed, and the volume removed is
/// the section's 2π∫r dA per turn over the turns inside the shaft.
#[test]
fn faceted_cut_of_a_helical_groove() {
    let shaft = cylinder(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 1.5, 4.0).unwrap();
    let (p, k) = (0.5, Vec3::new(0.0, 0.0, 1.0));
    // From a turn under the shaft to its middle: 4.5 turns, the first half-turn in the air.
    let base = Vec3::new(0.0, 0.0, -p);
    let plane = Plane { origin: base, x: Vec3::new(1.0, 0.0, 0.0), y: k };
    let section = [Vec2::new(1.2, -0.06), Vec2::new(1.6, -0.06), Vec2::new(1.6, 0.06), Vec2::new(1.2, 0.06)];
    let groove = sweep_helix_mesh(&plane, &section, base, k, p, 5.0, 48).unwrap();
    let cut = faceted_boolean(&shaft, &groove, BoolOp::Cut, 1e-3).unwrap().unwrap();
    let m = cut.tessellate(0.01).unwrap().measure();
    // In the shaft from z = 0: the section (r 1.2…1.5, 0.12 tall) for 4 turns, plus the part of
    // the first turn's section above z = 0 (its z range is ±0.06 about the turn's height).
    let per_turn = 2.0 * PI * 0.12 * (1.5f64.powi(2) - 1.2f64.powi(2)) / 2.0;
    let removed = PI * 2.25 * 4.0 - m.volume;
    assert!((removed - 4.0 * per_turn).abs() < 0.03 * per_turn * 4.0, "{removed} vs {}", 4.0 * per_turn);
}

/// A groove cut into the wall of a bore through a block (a nut): it runs out of both ends.
#[test]
fn faceted_cut_into_a_bore() {
    let nut = crate::box_solid(Vec3::new(-3.0, -3.0, 0.0), Vec3::new(3.0, 3.0, 2.0)).unwrap();
    let bore = cylinder(Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 1.0), 1.23, 4.0).unwrap();
    let nut = crate::boolean(&nut, &bore, BoolOp::Cut).unwrap().unwrap();
    let v0 = nut.tessellate(1e-3).unwrap().measure().volume;
    let (p, k) = (0.5, Vec3::new(0.0, 0.0, 1.0));
    let base = Vec3::new(0.0, 0.0, -p);
    let plane = Plane { origin: base, x: Vec3::new(1.0, 0.0, 0.0), y: k };
    let section = [Vec2::new(1.1, -0.1), Vec2::new(1.5, -0.1), Vec2::new(1.5, 0.1), Vec2::new(1.1, 0.1)];
    let groove = sweep_helix_mesh(&plane, &section, base, k, p, 6.0, 48).unwrap();
    let cut = faceted_boolean(&nut, &groove, BoolOp::Cut, 2.5e-3).unwrap().unwrap();
    let removed = v0 - cut.tessellate(1e-3).unwrap().measure().volume;
    // Four turns of the section's part outside the bore (r 1.23…1.5, 0.2 tall).
    let want = 4.0 * PI * 0.2 * (1.5f64.powi(2) - 1.23f64.powi(2));
    assert!((removed - want).abs() < 0.03 * want, "{removed} vs {want}");
}

/// #1250: a long thread (48 turns of an M8 groove on a 60 mm shaft). The groove's facets come in
/// sweep order; taken as BSP splitters in that order they made the tree as deep as the facet
/// list ("planar boolean: tree too deep", after minutes and gigabytes). It cuts now, closed.
#[test]
fn a_long_thread_cuts_without_a_deep_tree() {
    let (r, len, p) = (4.0, 60.0, 1.25);
    let shaft = cylinder(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), r, len).unwrap();
    let k = Vec3::new(0.0, 0.0, 1.0);
    let base = Vec3::new(0.0, 0.0, -p);
    let plane = Plane { origin: base, x: Vec3::new(1.0, 0.0, 0.0), y: k };
    let section = [Vec2::new(3.3, -0.08), Vec2::new(4.1, -0.5), Vec2::new(4.1, 0.5), Vec2::new(3.3, 0.08)];
    let groove = sweep_helix_mesh(&plane, &section, base, k, p, len / p + 2.0, 48).unwrap();
    let cut = faceted_boolean(&shaft, &groove, BoolOp::Cut, p * 0.005).unwrap().unwrap();
    let v = cut.tessellate(0.01).unwrap().measure().volume;
    let v0 = PI * r * r * len;
    assert!(v < v0 * 0.95 && v > v0 * 0.6, "{v} of {v0}");
}

fn helical(pitch: f64, depth: f64, from: f64, to: f64) -> crate::HelicalThread {
    crate::HelicalThread { pitch, depth, angle: PI / 3.0, clearance: 0.0, from, to, left_handed: false, start_angle: 0.0 }
}

/// Removed volume of a helical groove: its section times its centroid's path (Pappus).
fn groove_volume(t: &crate::HelicalThread, radius: f64, internal: bool) -> f64 {
    let s = t.section().unwrap();
    let into = if internal { 1.0 } else { -1.0 };
    let area = s.deep * (s.mouth + s.root) / 2.0;
    let centroid = radius + into * s.deep * (s.mouth + 2.0 * s.root) / (3.0 * (s.mouth + s.root));
    area * centroid * 2.0 * PI * t.turns().unwrap()
}

/// An exact helical thread on a shaft: the cylinder keeps its surface, trimmed by the groove's
/// mouth, and five faces are added (three helicoid walls, two end caps). It is a sound solid,
/// it meshes for display, and it removes the groove's Pappus volume.
#[test]
fn helical_brep_thread_on_a_shaft() {
    let shaft = cylinder(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 4.0, 20.0).unwrap();
    let at = Vec3::new(4.0, 0.0, 10.0);
    let cyl = crate::cylinder_face_at(&shaft, at).unwrap();
    let (s0, s1) = (cyl.start.min(cyl.end), cyl.start.max(cyl.end));
    let t = helical(1.25, 0.68, s0 + 2.0, s1 - 2.0);
    let b = crate::helical_thread(&shaft, at, &cyl, &t).unwrap();
    assert!(b.validity().is_empty(), "{:?}", b.validity());
    assert_eq!(b.face_count(), shaft.face_count() + 4, "the two halves of the cylinder are one face, and five groove faces");
    let removed = shaft.tessellate(1e-3).unwrap().measure().volume - b.tessellate(1e-3).unwrap().measure().volume;
    let want = groove_volume(&t, 4.0, false);
    assert!((removed - want).abs() < 0.01 * want, "{removed} vs {want}");
    let plan = b.display_plan(0.01).unwrap();
    assert!(!plan.faces.is_empty());
    // Still a cylinder of the shaft's radius where the thread is not.
    assert!(crate::cylinder_face_at(&b, Vec3::new(4.0, 0.0, 0.5)).is_some());
}

/// The same groove in a nut's bore runs outwards into the wall; left-handed it turns the
/// other way, and a clearance takes more material away.
#[test]
fn helical_brep_thread_in_a_bore_with_clearance() {
    let block = crate::box_solid(Vec3::new(-8.0, -8.0, 0.0), Vec3::new(8.0, 8.0, 10.0)).unwrap();
    let bore = cylinder(Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 1.0), 3.4, 12.0).unwrap();
    let nut = crate::boolean(&block, &bore, BoolOp::Cut).unwrap().unwrap();
    let at = Vec3::new(-3.4, 0.0, 5.0);
    let cyl = crate::cylinder_face_at(&nut, at).unwrap();
    assert!(cyl.internal);
    let (s0, s1) = (cyl.start.min(cyl.end), cyl.start.max(cyl.end));
    let v0 = nut.tessellate(1e-3).unwrap().measure().volume;
    let mut last = 0.0;
    for (clearance, left_handed) in [(0.0, false), (0.08, true)] {
        let t = crate::HelicalThread { clearance, left_handed, ..helical(1.25, 0.68, s0 + 1.0, s1 - 1.0) };
        let b = crate::helical_thread(&nut, at, &cyl, &t).unwrap();
        assert!(b.validity().is_empty(), "{:?}", b.validity());
        let removed = v0 - b.tessellate(1e-3).unwrap().measure().volume;
        let want = groove_volume(&t, 3.4, true);
        assert!((removed - want).abs() < 0.01 * want, "{removed} vs {want}");
        assert!(removed > last);
        last = removed;
    }
}

/// What can't be a thread is refused with a reason, never a broken body.
#[test]
fn helical_brep_thread_refusals() {
    let shaft = cylinder(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 4.0, 20.0).unwrap();
    let at = Vec3::new(4.0, 0.0, 10.0);
    let cyl = crate::cylinder_face_at(&shaft, at).unwrap();
    let (s0, s1) = (cyl.start.min(cyl.end), cyl.start.max(cyl.end));
    for t in [
        helical(1.25, 0.68, s0, s1),
        helical(1.25, 3.0, s0 + 2.0, s1 - 2.0),
        helical(1.25, 0.68, s0 + 2.0, s0 + 2.5),
        helical(f64::NAN, 0.68, s0 + 2.0, s1 - 2.0),
        crate::HelicalThread { clearance: 0.5, ..helical(1.25, 0.68, s0 + 2.0, s1 - 2.0) },
        crate::HelicalThread { angle: 0.0, ..helical(1.25, 0.68, s0 + 2.0, s1 - 2.0) },
    ] {
        assert!(crate::helical_thread(&shaft, at, &cyl, &t).is_err(), "{t:?}");
    }
    // A tube: the groove would break through its wall.
    let tube =
        crate::boolean(&shaft, &cylinder(Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, 0.0, 1.0), 3.6, 22.0).unwrap(), BoolOp::Cut).unwrap().unwrap();
    let cyl = crate::cylinder_face_at(&tube, at).unwrap();
    assert!(crate::helical_thread(&tube, at, &cyl, &helical(1.25, 0.68, s0 + 2.0, s1 - 2.0)).is_err());
}

/// A clearance on a large shaft: the material check samples well inside the material, not
/// just under the surface where the check mesh's facets can miss it (seen on a Ø33.25 shaft).
#[test]
fn helical_brep_thread_with_clearance_on_a_large_shaft() {
    let shaft = cylinder(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 16.625, 20.0).unwrap();
    let at = Vec3::new(16.625, 0.0, 10.0);
    let cyl = crate::cylinder_face_at(&shaft, at).unwrap();
    let (s0, s1) = (cyl.start.min(cyl.end), cyl.start.max(cyl.end));
    let depth = crate::HelicalThread::basic_depth(1.25, PI / 3.0, 0.1);
    let t = crate::HelicalThread { clearance: 0.1, ..helical(1.25, depth, s0 + 2.0, s1 - 4.0) };
    let b = crate::helical_thread(&shaft, at, &cyl, &t).unwrap();
    assert!(b.validity().is_empty(), "{:?}", b.validity());
}

/// The largest depth a refusal names fits: with a clearance the crest, not the root, limits it.
#[test]
fn helical_brep_thread_refusal_names_a_depth_that_fits() {
    for clearance in [0.0, 0.15] {
        let t = crate::HelicalThread { clearance, ..helical(1.25, 1.5, 0.0, 10.0) };
        let e = t.section().unwrap_err().to_string();
        let most: f64 = e.split("at most ").nth(1).and_then(|r| r.split(' ').next()).unwrap().parse().unwrap();
        assert!(crate::HelicalThread { depth: most * 0.999, ..t }.section().is_ok(), "{e}");
        assert!(crate::HelicalThread { depth: most * 1.02, ..t }.section().is_err(), "{e}");
    }
    assert!(crate::HelicalThread { clearance: 0.6, ..helical(1.25, 0.5, 0.0, 10.0) }.section().unwrap_err().to_string().contains("clearance"));
}

/// The helices stay on the true helix at the stations a groove gets (worst at its ends: 7e-4
/// of the radius at 16 a turn, 4e-3 at the most turns a groove may make), and their knots stay
/// within what the tessellation planner evaluates.
#[test]
fn helical_brep_helices_follow_the_helix() {
    use truck_modeling::ParametricCurve;
    let (r, pitch) = (5.0f64, 1.5f64);
    for (turns, within) in [(6.0, 1e-3), (145.0, 1e-3), (400.0, 5e-3)] {
        let m = crate::thread_brep::helix_stations(turns);
        let pts: Vec<Vec3> = (0..m)
            .map(|i| {
                let a = 2.0 * PI * turns * i as f64 / (m - 1) as f64;
                Vec3::new(r * a.cos(), r * a.sin(), pitch * a / (2.0 * PI))
            })
            .collect();
        let c = crate::thread_brep::helix_spline(&pts).unwrap();
        assert!(c.knot_vec().len() <= solidcraft_geom::tess::MAX_EVAL_KNOTS, "{turns} turns: {} knots", c.knot_vec().len());
        let worst = (0..=20_000)
            .map(|i| {
                let p = c.subs(i as f64 / 20_000.0);
                let a = 2.0 * PI * p.z / pitch;
                let on = Vec3::new(r * a.cos(), r * a.sin(), p.z);
                (Vec3::new(p.x, p.y, p.z) - on).len()
            })
            .fold(0.0f64, f64::max);
        assert!(worst < within * r, "{turns} turns: {worst} mm off the helix");
    }
}

/// A lead screw's thread: M10×1.5 down 220 mm, about 145 turns. The tessellation planner
/// meshes it (its helices once had more knots than the planner evaluates, and the reference
/// mesh of the fallback came out inside out), and it removes the groove's volume. Measured at a
/// micron (more nodes on a helix than the planner takes), it is meshed coarser by the planner,
/// not the reference way; and it is seen to turn more than once (16 samples of its helix once
/// stepped 9.03 turns each and read as a fifth of a radian).
#[test]
fn helical_brep_thread_of_many_turns() {
    let shaft = cylinder(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 5.0, 220.0).unwrap();
    let at = Vec3::new(5.0, 0.0, 110.0);
    let cyl = crate::cylinder_face_at(&shaft, at).unwrap();
    let (s0, s1) = (cyl.start.min(cyl.end), cyl.start.max(cyl.end));
    let t = helical(1.5, 0.81, s0 + 1.5, s1 - 1.5);
    assert!(t.turns().unwrap() > 140.0);
    let b = crate::helical_thread(&shaft, at, &cyl, &t).unwrap();
    let planned = |b: &crate::Body| crate::tess::plan(&b.solid, 0.01, None, b.boundary_tol).unwrap().0.evaluate().measure().volume;
    let removed = planned(&shaft) - planned(&b);
    let want = groove_volume(&t, 5.0, false);
    assert!((removed - want).abs() < 0.01 * want, "{removed} vs {want}");
    assert!(crate::tess::seam::turns_more_than_once(&b.solid));
    let fine = b.tessellate_with(1.26e-3, true).unwrap().measure().volume;
    assert!((fine - planned(&b)).abs() < 0.002 * fine, "{fine} vs {}", planned(&b));
}

/// A start angle turns the groove about the axis, which is the same as moving it along: a
/// quarter turn puts the grooves a quarter pitch from where they were (along any line on the
/// face), and the thread removes the same volume.
#[test]
fn helical_brep_thread_start_angle_turns_the_groove() {
    let shaft = cylinder(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 4.0, 20.0).unwrap();
    let at = Vec3::new(4.0, 0.0, 10.0);
    let cyl = crate::cylinder_face_at(&shaft, at).unwrap();
    let (s0, s1) = (cyl.start.min(cyl.end), cyl.start.max(cyl.end));
    let p = 1.25;
    let t = helical(p, 0.68, s0 + 2.0, s1 - 2.0);
    let a = crate::helical_thread(&shaft, at, &cyl, &t).unwrap();
    let b = crate::helical_thread(&shaft, at, &cyl, &crate::HelicalThread { start_angle: PI / 2.0, ..t }).unwrap();
    let (ma, mb) = (a.tessellate(1e-3).unwrap(), b.tessellate(1e-3).unwrap());
    let (va, vb) = (ma.measure().volume, mb.measure().volume);
    assert!((va - vb).abs() < 1e-3 * va, "{va} vs {vb}");
    // The middle of the first whole groove up a line 0.2 mm under the face, from z = 8.
    let groove = |m: &solidcraft_geom::Mesh| {
        let empty: Vec<bool> = (0..=3000).map(|i| !m.contains(Vec3::new(3.8, 0.0, 8.0 + i as f64 * 1e-3))).collect();
        let start = (1..empty.len()).find(|&i| empty[i] && !empty[i - 1]).unwrap();
        let end = (start..empty.len()).find(|&i| !empty[i]).unwrap();
        8.0 + (start + end - 1) as f64 * 0.5e-3
    };
    let shift = (groove(&mb) - groove(&ma)).rem_euclid(p);
    assert!((shift - p / 4.0).abs() < 0.02 || (shift - 3.0 * p / 4.0).abs() < 0.02, "moved {shift} mm, not a quarter pitch");
}
