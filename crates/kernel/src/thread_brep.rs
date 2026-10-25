//! Modelled threads as exact helical B-rep: a groove screwed into a cylindrical face, built
//! face by face rather than cut by a boolean. The groove's walls are helicoids (ruled B-spline
//! surfaces between helices), its two ends planar caps, and the threaded face keeps its own
//! cylinder, trimmed by the groove's mouth: two helices and the caps' edges on the cylinder.
//! Nothing else of the body changes, so the groove has to lie inside the face (a thread that
//! starts after the face's start and finishes before its end).

use solidcraft_geom::{Mesh, Vec3};
use truck_modeling as mt;

use crate::body::{Body, Solid, p3};
use crate::history::{Change, FaceOrigin, FaceProv};
use crate::record::Operand;
use crate::topo::CylinderFace;
use crate::{FaceSurface, KernelError, Result, guard};
use mt::builder;

/// A helical groove's shape and where it runs along a cylindrical face.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HelicalThread {
    /// Axial advance per turn (mm).
    pub pitch: f64,
    /// Radial depth of the groove into the material (mm), before clearance.
    pub depth: f64,
    /// Included angle between the flanks (radians; 60° for metric threads).
    pub angle: f64,
    /// How far every threaded surface (both flanks and the root) moves into the material, to
    /// leave room for the mating part (mm, 0 or more). The crests stay on the face.
    pub clearance: f64,
    /// The axial extent of the groove's mouth on the face, measured along the cylinder's axis
    /// from its `axis_point` (as [`CylinderFace::start`] and `end` are).
    pub from: f64,
    pub to: f64,
    pub left_handed: bool,
}

/// The groove's section in the axial plane: its width at the face (`mouth`) and at the
/// bottom (`root`), and its radial depth (`deep`, clearance included).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrooveSection {
    pub mouth: f64,
    pub root: f64,
    pub deep: f64,
}

/// Stations per turn of the helices' interpolation: off the true helix by about 1e-6 of the
/// radius.
const STATIONS_PER_TURN: f64 = 48.0;
/// Most turns a groove may make.
const MAX_TURNS: f64 = 400.0;

fn fail(what: &str) -> KernelError {
    KernelError::Failed(format!("helical thread: {what}"))
}

impl HelicalThread {
    /// The section, or why this pitch, depth, angle and clearance make no thread. The section
    /// is symmetric: groove and tooth are equally wide halfway down the basic depth, as in the
    /// basic profile of the metric and unified threads.
    pub fn section(&self) -> Result<GrooveSection> {
        let (p, d, a, c) = (self.pitch, self.depth, self.angle, self.clearance);
        if !(p.is_finite() && d.is_finite() && a.is_finite() && c.is_finite()) {
            return Err(KernelError::Invalid("thread dimensions must be finite".into()));
        }
        if !(p > 1e-3 && p < 1e4) {
            return Err(KernelError::Invalid("the pitch must be positive".into()));
        }
        if !(d > 1e-4) {
            return Err(KernelError::Invalid("the thread depth must be positive".into()));
        }
        if !(c >= 0.0) {
            return Err(KernelError::Invalid("the clearance cannot be negative".into()));
        }
        let max_angle = 170f64.to_radians();
        if !(a > 0.0 && a <= max_angle) {
            return Err(KernelError::Invalid("the thread angle must be between 0° and 170°".into()));
        }
        let (tan, cos) = ((a / 2.0).tan(), (a / 2.0).cos());
        // Each flank moves `c` square to itself, so `c / cos` along the axis; the root `c` deeper.
        let mouth = p / 2.0 + d * tan + 2.0 * c / cos;
        let deep = d + c;
        let root = mouth - 2.0 * deep * tan;
        // The deepest groove that still has a root (the flanks not meeting) and a crest (the
        // groove narrower than the pitch): with a clearance the crest is usually the tighter.
        let rooted = (p * 0.49 + 2.0 * c * (1.0 / cos - tan)) / tan;
        let crested = (p * 0.48 - 2.0 * c / cos) / tan;
        let most = rooted.min(crested);
        if root < p * 0.01 || mouth > p * 0.98 {
            if most <= 1e-4 {
                return Err(KernelError::Invalid(format!(
                    "a {c:.3} mm clearance leaves no thread at a {p:.3} mm pitch: make the clearance smaller or the pitch coarser"
                )));
            }
            // The limit that binds: the one the deepest fitting groove reaches first.
            let why = if crested < rooted { "no crest is left between the turns" } else { "the flanks meet before the root" };
            return Err(KernelError::Invalid(format!(
                "a {d:.3} mm deep thread does not fit this pitch, angle and clearance: {why} (at most {most:.3} mm deep)"
            )));
        }
        Ok(GrooveSection { mouth, root, deep })
    }

    /// The depth a thread gets when none is given: the basic depth of a 60° thread (5/8 of the
    /// fundamental triangle's height), scaled to the angle, made shallower where a clearance
    /// would otherwise leave less than a tenth of the pitch as crest.
    pub fn basic_depth(pitch: f64, angle: f64, clearance: f64) -> f64 {
        let tan = (angle / 2.0).tan();
        let basic = 5.0 / 16.0 * pitch / tan;
        let crest = (0.4 * pitch - 2.0 * clearance / (angle / 2.0).cos()) / tan;
        basic.min(crest)
    }

    /// Turns the groove makes between `from` and `to`.
    pub fn turns(&self) -> Result<f64> {
        let s = self.section()?;
        Ok((self.to - self.from - s.mouth) / self.pitch)
    }
}

/// The helices' clamped cubic B-spline through `pts` at uniform parameters (knots averaged),
/// solved as the banded system it is: a long thread has thousands of stations.
fn helix_spline(pts: &[Vec3]) -> Option<mt::BSplineCurve<mt::Point3>> {
    let m = pts.len();
    if m < 4 {
        return None;
    }
    let u: Vec<f64> = (0..m).map(|i| i as f64 / (m - 1) as f64).collect();
    let mut knots = vec![0.0; 4];
    for j in 1..(m - 3) {
        knots.push((j..j + 3).filter_map(|i| u.get(i)).sum::<f64>() / 3.0);
    }
    knots.extend([1.0; 4]);
    // Row i holds columns i-3..=i+3 at a[i][0..7].
    let mut a = vec![[0.0f64; 7]; m];
    let mut q: Vec<[f64; 3]> = pts.iter().map(|p| [p.x, p.y, p.z]).collect();
    for (i, ui) in u.iter().enumerate() {
        let span = (3..m).rev().find(|&s| knots.get(s).is_some_and(|k| *k <= *ui))?;
        let span = if *ui >= 1.0 { m - 1 } else { span };
        let basis = cubic_basis(&knots, span, *ui)?;
        for (k, b) in basis.iter().enumerate() {
            let col = span + k - 3;
            let at = (col + 3).checked_sub(i).filter(|x| *x < 7)?;
            *a.get_mut(i)?.get_mut(at)? = *b;
        }
    }
    // A B-spline collocation matrix is totally positive: no pivoting.
    for col in 0..m {
        let piv = a.get(col)?.get(3).copied()?;
        if piv.abs() < 1e-14 {
            return None;
        }
        let prow = *a.get(col)?;
        let qc = *q.get(col)?;
        for r in col + 1..(col + 4).min(m) {
            let f = a.get(r)?.get(col + 3 - r).copied()? / piv;
            if f == 0.0 {
                continue;
            }
            for c in col..(col + 4).min(m) {
                let v = prow.get(c + 3 - col).copied()?;
                *a.get_mut(r)?.get_mut(c + 3 - r)? -= f * v;
            }
            let qr = q.get_mut(r)?;
            for k in 0..3 {
                qr[k] -= f * qc[k];
            }
        }
    }
    let mut x = vec![[0.0f64; 3]; m];
    for i in (0..m).rev() {
        let row = a.get(i)?;
        let mut v = *q.get(i)?;
        for c in i + 1..(i + 4).min(m) {
            let xc = *x.get(c)?;
            let aic = row.get(c + 3 - i).copied()?;
            for k in 0..3 {
                v[k] -= aic * xc[k];
            }
        }
        let d = row.get(3).copied()?;
        *x.get_mut(i)? = [v[0] / d, v[1] / d, v[2] / d];
    }
    let ctrl: Vec<mt::Point3> = x.iter().map(|c| mt::Point3::new(c[0], c[1], c[2])).collect();
    mt::BSplineCurve::try_new(mt::KnotVec::from(knots), ctrl).ok()
}

/// The four cubic basis functions non-zero on knot span `span` at `u` (Cox–de Boor).
fn cubic_basis(knots: &[f64], span: usize, u: f64) -> Option<[f64; 4]> {
    let mut n = [1.0, 0.0, 0.0, 0.0];
    let mut left = [0.0; 4];
    let mut right = [0.0; 4];
    for j in 1..4 {
        left[j] = u - knots.get(span + 1 - j)?;
        right[j] = knots.get(span + j)? - u;
        let mut saved = 0.0;
        for r in 0..j {
            let den = right[r + 1] + left[j - r];
            let t = if den.abs() < 1e-300 { 0.0 } else { n[r] / den };
            n[r] = saved + right[r + 1] * t;
            saved = left[j - r] * t;
        }
        n[j] = saved;
    }
    Some(n)
}

/// The ruled surface between two helices on one knot vector (the screw motion of a straight
/// profile segment is ruled exactly so).
fn ruled(a: &mt::BSplineCurve<mt::Point3>, b: &mt::BSplineCurve<mt::Point3>) -> Result<mt::BSplineSurface<mt::Point3>> {
    let rows: Vec<Vec<mt::Point3>> = a.control_points().iter().zip(b.control_points()).map(|(p, q)| vec![*p, *q]).collect();
    mt::BSplineSurface::try_new((a.knot_vec().clone(), mt::KnotVec::bezier_knot(1)), rows).map_err(|e| fail(&e.to_string()))
}

/// The faces of `body` on the cylinder `cyl` (a cylinder is often made of two halves), and
/// the one of them at `at`.
fn threaded_faces(body: &Body, mesh: &Mesh, cyl: &CylinderFace, at: Vec3) -> Option<(usize, Vec<usize>)> {
    let tol = (body.size() * 1e-4).max(1e-6);
    let on_cylinder = |f: usize| match crate::face_surface(body, f) {
        Some(FaceSurface::Cylinder { origin, axis, radius }) => {
            let d = origin - cyl.axis_point;
            (radius - cyl.radius).abs() <= tol && axis.cross(*cyl.axis).len() <= 1e-6 && (d - cyl.axis * d.dot(cyl.axis)).len() <= tol
        }
        _ => false,
    };
    let all: Vec<usize> = (0..body.face_count()).filter(|f| on_cylinder(*f)).collect();
    let face = nearest_face(mesh, at, |f| all.contains(&(f as usize))).map(|(f, _)| f as usize)?;
    Some((face, all))
}

/// `faces` (one shell's, on one surface, each the next's neighbour) as one face: their loops,
/// less the edges between them, chained again. A cylinder's two halves become the cylinder
/// between its two end circles, with no seam.
fn merged_loops(faces: &[&mt::Face]) -> Result<Vec<mt::Wire>> {
    let mut uses: std::collections::HashMap<mt::EdgeID, usize> = std::collections::HashMap::new();
    for f in faces {
        for w in f.boundaries() {
            for e in w.edge_iter() {
                *uses.entry(e.id()).or_default() += 1;
            }
        }
    }
    let mut left: Vec<mt::Edge> = faces
        .iter()
        .flat_map(|f| f.boundaries())
        .flat_map(|w| w.edge_iter().cloned().collect::<Vec<_>>())
        .filter(|e| uses.get(&e.id()) == Some(&1))
        .collect();
    let mut loops = Vec::new();
    while let Some(first) = left.pop() {
        let start = first.front().id();
        let mut at = first.back().id();
        let mut wire = vec![first];
        while at != start {
            let i = left.iter().position(|e| e.front().id() == at).ok_or_else(|| fail("the face's loops do not close"))?;
            let e = left.swap_remove(i);
            at = e.back().id();
            wire.push(e);
            if wire.len() > 100_000 {
                return Err(fail("loop"));
            }
        }
        loops.push(wire.into());
    }
    Ok(loops)
}

/// The face of the triangle nearest `p` among faces `ok` accepts, and how far it is.
fn nearest_face(mesh: &Mesh, p: Vec3, ok: impl Fn(u32) -> bool) -> Option<(u32, f64)> {
    let mut best: Option<(u32, f64)> = None;
    let mut cache: std::collections::HashMap<u32, bool> = std::collections::HashMap::new();
    for (t, f) in mesh.triangles.iter().zip(&mesh.tri_face) {
        if !*cache.entry(*f).or_insert_with(|| ok(*f)) {
            continue;
        }
        let Some([a, b, c]) = mesh.tri(t) else { continue };
        let d = crate::boolean_guard::point_triangle_dist(p, a, b, c);
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((*f, d));
        }
    }
    best
}

/// Screw a helical groove into the cylindrical face of `body` at `at` (`cyl` is that face, as
/// [`crate::cylinder_face_at`] gives it): an external thread on a shaft, an internal one in a
/// hole. The result is exact B-rep, its groove faces named as made from the threaded face.
pub fn helical_thread(body: &Body, at: Vec3, cyl: &CylinderFace, t: &HelicalThread) -> Result<Body> {
    body.require_brep("a thread")?;
    if body.solid.boundaries().len() != 1 {
        return Err(KernelError::Invalid("not supported yet: a helical thread on a body of several shells (a hollow or a multi-piece body)".into()));
    }
    let s = t.section()?;
    let turns = t.turns()?;
    let (r, k) = (cyl.radius, cyl.axis);
    if !(r.is_finite() && r > 1e-3 && t.from.is_finite() && t.to.is_finite()) {
        return Err(KernelError::Invalid("thread placement".into()));
    }
    if !(turns >= 0.25) {
        return Err(KernelError::Invalid(format!(
            "the threaded length is too short for a {:.3} mm pitch: it needs at least {:.3} mm",
            t.pitch,
            s.mouth + 0.25 * t.pitch
        )));
    }
    if turns > MAX_TURNS {
        return Err(KernelError::Invalid(format!("a thread is limited to {MAX_TURNS:.0} turns")));
    }
    let land = (t.pitch * 1e-3).max(1e-4);
    if t.from < cyl.start.min(cyl.end) + land || t.to > cyl.start.max(cyl.end) - land {
        return Err(KernelError::Invalid("the thread must start after the face's start and finish before its end".into()));
    }
    // Into the material: inwards for a shaft, outwards in a hole.
    let into = if cyl.internal { 1.0 } else { -1.0 };
    let r_root = r + into * s.deep;
    if !(r_root > r * 0.05) {
        return Err(KernelError::Invalid(format!("a {:.3} mm deep thread does not fit a {:.3} mm shaft", s.deep, 2.0 * r)));
    }
    let u = k.cross(if k.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) });
    let u = u.normalized().ok_or_else(|| fail("axis"))?;
    let v = k.cross(u);
    let hand = if t.left_handed { -1.0 } else { 1.0 };
    let total = std::f64::consts::TAU * turns;
    let centre0 = t.from + s.mouth / 2.0;
    // The point `along` the axis from the groove's centre line and `radius` out, `a` radians
    // along the helix.
    let at_helix = |a: f64, radius: f64, along: f64| {
        let th = hand * a;
        cyl.axis_point + k * (centre0 + t.pitch * a / std::f64::consts::TAU + along) + (u * th.cos() + v * th.sin()) * radius
    };
    // The section's corners: mouth (on the face) and root, below and above the centre line.
    let corners = [(r, -s.mouth / 2.0), (r_root, -s.root / 2.0), (r_root, s.root / 2.0), (r, s.mouth / 2.0)];

    let size = body.size();
    let mesh = body.tessellate((size * 1e-3).max(1e-3).min(t.pitch * 0.05))?;
    let (face, on_cylinder) =
        threaded_faces(body, &mesh, cyl, at).ok_or_else(|| KernelError::Invalid("no cylindrical face there to thread".into()))?;
    // The mouth must lie on that face and the groove in the material: nothing else of the body
    // (a cross hole, a flat, a thin wall) may meet it.
    let checks = ((turns * 12.0).ceil() as usize).clamp(8, 4000);
    let near = (size * 2e-3).max(t.pitch * 0.02);
    for i in 0..=checks {
        let a = total * i as f64 / checks as f64;
        for along in [-s.mouth / 2.0, s.mouth / 2.0] {
            let p = at_helix(a, r, along * 0.98);
            match nearest_face(&mesh, p, |_| true) {
                Some((f, d)) if on_cylinder.contains(&(f as usize)) && d <= near => {}
                _ => return Err(KernelError::Invalid("something else of the body meets the thread: it must run on the face alone".into())),
            }
        }
        let mid = (r + r_root) / 2.0;
        // Samples well inside the material (the check mesh's facets lie up to its tolerance
        // inside curved faces): halfway down the groove and past its root.
        let past = (s.deep * 0.25).max(t.pitch * 0.1);
        for (radius, along) in [(mid, 0.0), (mid, (s.mouth + s.root) * 0.2), (mid, -(s.mouth + s.root) * 0.2), (r_root + into * past, 0.0)] {
            if !mesh.contains(at_helix(a, radius, along)) {
                let what =
                    if cyl.internal { "the wall around the hole is too thin for this thread" } else { "the shaft is hollow under this thread" };
                return Err(KernelError::Invalid(what.into()));
            }
        }
    }

    let stations = ((turns * STATIONS_PER_TURN).ceil() as usize).max(8) + 1;
    let helices: Vec<mt::BSplineCurve<mt::Point3>> = corners
        .iter()
        .map(|(radius, along)| {
            let pts: Vec<Vec3> = (0..stations).map(|i| at_helix(total * i as f64 / (stations - 1) as f64, *radius, *along)).collect();
            helix_spline(&pts).ok_or_else(|| fail("helix"))
        })
        .collect::<Result<_>>()?;
    let walls: Vec<mt::BSplineSurface<mt::Point3>> = helices
        .windows(2)
        .map(|w| match w {
            [a, b] => ruled(a, b),
            _ => Err(fail("walls")),
        })
        .collect::<Result<_>>()?;
    // Before and after measured alike, so the difference is the groove's alone.
    let measure_tol = (size * 1e-3).max(1e-3).min(t.pitch * 0.02);
    let volume_before = body.tessellate(measure_tol)?.measure().volume;
    // Pappus: the groove's section times the path of its centroid.
    let area = s.deep * (s.mouth + s.root) / 2.0;
    let centroid = r + into * s.deep * (s.mouth + 2.0 * s.root) / (3.0 * (s.mouth + s.root));
    let removed = area * centroid * total;

    guard("helical thread", || {
        let faces: Vec<mt::Face> = body.solid.face_iter().cloned().collect();
        let f = faces.get(face).ok_or_else(|| fail("face"))?;
        // The picked face and its neighbours on the same cylinder, joined into one face.
        let mut members = vec![face];
        loop {
            let edges: std::collections::HashSet<mt::EdgeID> =
                members.iter().filter_map(|i| faces.get(*i)).flat_map(|x| x.edge_iter().map(|e| e.id()).collect::<Vec<_>>()).collect();
            let more: Vec<usize> = on_cylinder
                .iter()
                .copied()
                .filter(|i| !members.contains(i) && faces.get(*i).is_some_and(|x| x.edge_iter().any(|e| edges.contains(&e.id()))))
                .collect();
            if more.is_empty() {
                break;
            }
            members.extend(more);
        }
        members.sort_unstable();
        let joined: Vec<&mt::Face> = members.iter().filter_map(|i| faces.get(*i)).collect();
        let loops = if members.len() == 1 { f.boundaries() } else { merged_loops(&joined)? };
        let v0: Vec<mt::Vertex> = corners.iter().map(|(radius, along)| builder::vertex(p3(at_helix(0.0, *radius, *along)))).collect();
        let v1: Vec<mt::Vertex> = corners.iter().map(|(radius, along)| builder::vertex(p3(at_helix(total, *radius, *along)))).collect();
        let vert = |vs: &[mt::Vertex], i: usize| vs.get(i).cloned().ok_or_else(|| fail("vertex"));
        // Along the helices, start to end: mouth below, root below, root above, mouth above.
        let mut along = Vec::with_capacity(4);
        for (i, h) in helices.iter().enumerate() {
            along.push(mt::Edge::try_new(&vert(&v0, i)?, &vert(&v1, i)?, mt::Curve::BSplineCurve(h.clone())).map_err(|e| fail(&e.to_string()))?);
        }
        // Across the section at each end: corner i to corner i + 1, and the mouth (on the face)
        // from above back to below.
        let across = |vs: &[mt::Vertex]| -> Result<(Vec<mt::Edge>, mt::Edge)> {
            let sides = (0..3).map(|i| Ok(builder::line(&vert(vs, i)?, &vert(vs, i + 1)?))).collect::<Result<Vec<_>>>()?;
            Ok((sides, builder::line(&vert(vs, 3)?, &vert(vs, 0)?)))
        };
        let (start, mouth0) = across(&v0)?;
        let (end, mouth1) = across(&v1)?;
        let edge = |es: &[mt::Edge], i: usize| es.get(i).cloned().ok_or_else(|| fail("edge"));
        // Each wall counter-clockwise in its (along the helix, across the section) parameters.
        let mut groove: Vec<mt::Face> = Vec::with_capacity(5);
        for (i, w) in walls.iter().enumerate() {
            let wire: mt::Wire = vec![edge(&along, i)?, edge(&end, i)?, edge(&along, i + 1)?.inverse(), edge(&start, i)?.inverse()].into();
            groove.push(mt::Face::try_new(vec![wire], mt::Surface::BSplineSurface(w.clone())).map_err(|e| fail(&e.to_string()))?);
        }
        let cap = |w: mt::Wire| builder::try_attach_plane(&[w]).map_err(|e| fail(&format!("end cap: {e}")));
        let start_cap: mt::Wire = vec![edge(&start, 0)?, edge(&start, 1)?, edge(&start, 2)?, mouth0.clone()].into();
        groove.push(cap(start_cap)?);
        let end_cap: mt::Wire = vec![mouth1.inverse(), edge(&end, 2)?.inverse(), edge(&end, 1)?.inverse(), edge(&end, 0)?.inverse()].into();
        groove.push(cap(end_cap)?);
        // The mouth on the face, against the walls' and caps' use of its edges.
        let mouth: mt::Wire = vec![edge(&along, 3)?, mouth1.clone(), edge(&along, 0)?.inverse(), mouth0.inverse()].into();
        let operand = Operand::new(body);
        let trimmed_from = if members.len() == 1 {
            crate::record::from_face(0, face, Change::Modified)
        } else {
            FaceProv { origins: members.iter().map(|i| FaceOrigin::Face { operand: 0, face: *i }).collect(), change: Change::Merged }
        };
        let history: Option<Vec<FaceProv>> = (0..faces.len())
            .filter(|i| !members.contains(i))
            .map(|i| Some(crate::record::from_face(0, i, Change::Kept)))
            .chain([Some(trimmed_from)])
            .chain((0..groove.len()).map(|_| Some(crate::record::generated(vec![FaceOrigin::Face { operand: 0, face }]))))
            .collect();
        // The threaded face's surface: the whole cylinder, which the mouth's helices wind around
        // (the faces it replaces may each hold only a part of it, a half as a rational patch).
        let (lo, hi) = (cyl.start.min(cyl.end) - 1.0, cyl.start.max(cyl.end) + 1.0);
        let line = mt::Curve::Line(mt::Line(p3(cyl.axis_point + k * lo + u * r), p3(cyl.axis_point + k * hi + u * r)));
        let ak = mt::Vector3::new(k.x, k.y, k.z);
        let round = mt::Surface::RevolutedCurve(mt::Processor::new(mt::RevolutedCurve::by_revolution(line, p3(cyl.axis_point), ak)));
        let mut turned = round.clone();
        mt::Invertible::invert(&mut turned);
        // The likely orientations first, so one build is usually enough: the cylinder's normal
        // the way the face's points (out of the material), and the start cap's out of the
        // material into the groove, which runs on from it the way the helix turns.
        let mid = cyl.axis_point + k * ((lo + hi) / 2.0) + u * r;
        let out = if cyl.internal { -u } else { u };
        let normal_at = |srf: &mt::Surface, p: Vec3| {
            use mt::{ParametricSurface3D, SearchParameter};
            srf.search_parameter(p3(p), None, 100).map(|(a, b)| srf.normal(a, b)).map(|n| Vec3::new(n.x, n.y, n.z))
        };
        let round_first = normal_at(&round, mid).is_none_or(|n| n.dot(out) >= 0.0);
        let cap_into_groove = match groove.get(3).map(|c| c.oriented_surface()) {
            Some(mt::Surface::Plane(pl)) => {
                let n = pl.normal();
                Vec3::new(n.x, n.y, n.z).dot(v * hand) > 0.0
            }
            _ => true,
        };
        let (first, second) = if round_first { (&round, &turned) } else { (&turned, &round) };
        let flip = !cap_into_groove;
        let mut last = String::from("the faces do not close");
        for (flip, surface) in [(flip, first), (!flip, first), (flip, second), (!flip, second)] {
            let (g, m): (Vec<mt::Face>, mt::Wire) =
                if flip { (groove.iter().map(|x| x.inverse()).collect(), mouth.inverse()) } else { (groove.clone(), mouth.clone()) };
            let mut with_mouth = loops.clone();
            with_mouth.push(m);
            let trimmed = mt::Face::try_new(with_mouth, surface.clone()).map_err(|e| fail(&e.to_string()))?;
            let mut all: Vec<mt::Face> = faces.iter().enumerate().filter(|(i, _)| !members.contains(i)).map(|(_, x)| x.clone()).collect();
            all.push(trimmed);
            all.extend(g);
            let shells: Vec<mt::Shell> = vec![all.into()];
            let solid = match Solid::try_new(shells) {
                Ok(solid) => solid,
                Err(e) => {
                    last = e.to_string();
                    continue;
                }
            };
            // The body's own faces keep their outward orientation and the groove's follow them
            // edge by edge, so the solid needs no meshing to orient it (truck's mesher cannot
            // take a face trimmed by a helix of several turns).
            let b = Body::new_placed(solid, false, None, false)?;
            let bad = b.validity();
            if !bad.is_empty() {
                last = bad.join("; ");
                continue;
            }
            let after = b.tessellate(measure_tol)?.measure().volume;
            let cut = volume_before - after;
            if (cut - removed).abs() > 0.05 * removed + 1e-6 * volume_before.abs() {
                last = format!("the groove removes {cut:.4} mm³, not {removed:.4}");
                continue;
            }
            return crate::record::attach(b, std::slice::from_ref(&operand), history.clone(), Default::default());
        }
        Err(fail(&last))
    })
}
