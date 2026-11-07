//! SolidCraft (Android port): the helical thread, a groove of any pitch, depth, flank angle and
//! clearance screwed into a cylindrical face between two offsets from its ends, turned to a start
//! angle. Upstream's modelled threads cut standard forms; this cuts the port's custom groove the
//! same way, as an exact screw sweep (SMLib's, through solidcore) taken away by one boolean.

use super::*;
use crate::thread_forms::ThreadHand;

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
    /// from its `axis_point` (as [`kernel::CylinderFace::start`] and `end` are).
    pub from: f64,
    pub to: f64,
    pub left_handed: bool,
    /// The groove turned about the face's axis (radians, right-handed about the axis): where
    /// around the face it starts, to set its phase against a mating thread. Turning a thread by
    /// θ moves its crests θ/2π of a pitch along the axis; 0 starts it on a fixed direction.
    pub start_angle: f64,
}

/// The groove's section in the axial plane: its width at the face (`mouth`) and at the
/// bottom (`root`), and its radial depth (`deep`, clearance included).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrooveSection {
    pub mouth: f64,
    pub root: f64,
    pub deep: f64,
}

/// Most turns a groove may make (the screw sweep's limit is 200).
const MAX_TURNS: f64 = 190.0;

impl HelicalThread {
    /// The section, or why this pitch, depth, angle and clearance make no thread. The section
    /// is symmetric: groove and tooth are equally wide halfway down the basic depth, as in the
    /// basic profile of the metric and unified threads.
    pub fn section(&self) -> Result<GrooveSection> {
        let (p, d, a, c) = (self.pitch, self.depth, self.angle, self.clearance);
        if !(p.is_finite() && d.is_finite() && a.is_finite() && c.is_finite()) {
            return Err(DocError::Invalid("thread dimensions must be finite".into()));
        }
        if !(p > 1e-3 && p < 1e4) {
            return Err(DocError::Invalid("the pitch must be positive".into()));
        }
        if !(d > 1e-4) {
            return Err(DocError::Invalid("the thread depth must be positive".into()));
        }
        if !(c >= 0.0) {
            return Err(DocError::Invalid("the clearance cannot be negative".into()));
        }
        let max_angle = 170f64.to_radians();
        if !(a > 0.0 && a <= max_angle) {
            return Err(DocError::Invalid("the thread angle must be between 0° and 170°".into()));
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
                return Err(DocError::Invalid(format!(
                    "a {c:.3} mm clearance leaves no thread at a {p:.3} mm pitch: make the clearance smaller or the pitch coarser"
                )));
            }
            // The limit that binds: the one the deepest fitting groove reaches first.
            let why = if crested < rooted { "no crest is left between the turns" } else { "the flanks meet before the root" };
            return Err(DocError::Invalid(format!(
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

/// The body and cylindrical face a thread at `p` goes on.
fn thread_face(st: &ModelState, p: Vec3) -> Result<(String, kernel::CylinderFace)> {
    st.bodies
        .iter()
        .filter_map(|b| kernel::cylinder_face_at(&b.body, p).map(|c| (b.name.clone(), c)))
        .min_by(|a, b| {
            let dist = |c: &kernel::CylinderFace| ((p - c.axis_point - c.axis * (p - c.axis_point).dot(c.axis)).len() - c.radius).abs();
            dist(&a.1).total_cmp(&dist(&b.1))
        })
        .ok_or_else(|| DocError::Invalid("no cylindrical face there to thread".into()))
}

/// The threaded part of `cyl` along its axis, `length` from the end of the face nearest `p`
/// (the whole face without one), and whether that end is the face's `start`.
fn thread_span(cyl: &kernel::CylinderFace, p: Vec3, length: Option<f64>) -> Result<(f64, f64, bool)> {
    let s = (p - cyl.axis_point).dot(cyl.axis);
    let (mut a, mut b) = (cyl.start, cyl.end);
    let from_start = (s - a).abs() <= (b - s).abs();
    if let Some(l) = length {
        if !(l > 0.0) {
            return Err(DocError::Invalid("thread length must be positive".into()));
        }
        if from_start {
            b = (a + l).min(b);
        } else {
            a = (b - l).max(a);
        }
    }
    Ok((a, b, from_start))
}

/// The dimensions of a helical thread feature, read from its expressions.
pub struct HelicalInputs<'a> {
    pub designation: Option<&'a str>,
    pub length: Option<f64>,
    pub pitch: Option<f64>,
    pub depth: Option<f64>,
    pub angle: Option<f64>,
    pub clearance: Option<f64>,
    pub start_offset: Option<f64>,
    pub end_offset: Option<f64>,
    pub left_handed: bool,
    /// Radians; `None` = 0.
    pub start_angle: Option<f64>,
}

/// A helical thread on the face at `p`: a custom pitch on the face as it is, or an ISO metric
/// designation (its pitch, and the depth from the face to its root or major diameter).
/// Returns the body's name, the face, and the groove.
pub fn helical_thread_on(st: &ModelState, p: Vec3, h: &HelicalInputs) -> Result<(String, kernel::CylinderFace, HelicalThread)> {
    let (body, cyl) = thread_face(st, p)?;
    let (a, b, from_start) = thread_span(&cyl, p, h.length)?;
    let angle = h.angle.unwrap_or(std::f64::consts::PI / 3.0);
    let clearance = h.clearance.unwrap_or(0.0);
    let r = cyl.radius;
    let (pitch, depth) = match h.pitch {
        Some(pitch) => (pitch, h.depth.unwrap_or_else(|| HelicalThread::basic_depth(pitch, angle, clearance))),
        None => {
            let dia = 2.0 * r;
            let (major, pitch) = match h.designation {
                Some(s) => parse_metric_thread(s).ok_or_else(|| DocError::Invalid(format!("unknown thread `{s}` (ISO metric, like M8 or M8x1)")))?,
                None => crate::thread_forms::ThreadForm::Metric
                    .fit(dia, cyl.internal)
                    .ok_or_else(|| DocError::Invalid(format!("no metric thread fits a {dia:.2} mm cylinder")))?,
            };
            let m = crate::thread_forms::ThreadForm::Metric.format(major, pitch, pitch);
            let fit = |depth: f64| depth.min(HelicalThread::basic_depth(pitch, angle, clearance));
            let depth = match h.depth {
                Some(d) => d,
                None if cyl.internal => {
                    // Out to the major diameter, where the bolt's crests sit.
                    let d = major / 2.0 - r;
                    if !(d > pitch * 0.1) {
                        return Err(DocError::Invalid(format!(
                            "this {} mm hole is too wide for an {m} thread; drill {} mm for {m}, or use a custom thread on the hole as it is",
                            show_mm(dia),
                            show_mm(major - pitch)
                        )));
                    }
                    // A shallower groove would not reach the bolt's crests, and no bolt would go in:
                    // when the clearance leaves no crest between the turns at this hole's size, the
                    // hole must be wider (as a tap drill is for a printed part), not the thread shallower.
                    let most = HelicalThread::basic_depth(pitch, angle, clearance);
                    if d > most + 1e-9 {
                        let drill = (((major - 2.0 * most) * 100.0).ceil() / 100.0).max(dia);
                        return Err(DocError::Invalid(format!(
                            "with a {} mm clearance an {m} thread leaves no crest in a {} mm hole, and a shallower thread would not take the bolt: drill {} mm, or make the clearance smaller",
                            show_mm(clearance),
                            show_mm(dia),
                            show_mm(drill)
                        )));
                    }
                    d
                }
                None => {
                    // The shaft is the major diameter: down to the basic root.
                    if (dia - major).abs() > 0.02 * major {
                        return Err(DocError::Invalid(format!(
                            "a helical {m} thread needs a {} mm shaft (this one is {} mm): make the shaft that size, or use a custom thread on the shaft as it is",
                            show_mm(major),
                            show_mm(dia)
                        )));
                    }
                    fit(5.0 * pitch * 3f64.sqrt() / 16.0)
                }
            };
            (pitch, depth)
        }
    };
    if !(pitch.is_finite() && pitch > 0.0) {
        return Err(DocError::Invalid("the pitch must be positive".into()));
    }
    // By default a thread starts half a pitch in from the face's end and finishes half a pitch
    // before the other. A negative offset, to one pitch, runs the groove off that end into the
    // air, as a tap or die leaves it, so a nut or the end of a bolt has no partial turn of
    // material across its thread; at a shoulder it would cut into the shoulder. Zero would put
    // the groove's end on the face's end, which the boolean can't separate.
    let gap = |o: Option<f64>| -> Result<f64> {
        let o = o.unwrap_or(pitch / 2.0);
        if !(o.is_finite() && o >= -pitch && o.abs() >= pitch * 0.01) {
            return Err(DocError::Invalid(format!(
                "a helical thread's start and end offsets are positive (it starts after the face's end), or negative to one pitch ({} mm) to run off a free end, but not 0",
                show_mm(-pitch)
            )));
        }
        Ok(o)
    };
    let (near, far) = (gap(h.start_offset)?, gap(h.end_offset)?);
    let (from, to) = if from_start { (a + near, b - far) } else { (a + far, b - near) };
    if !(to > from) {
        return Err(DocError::Invalid("the start and end offsets leave nothing of the face to thread".into()));
    }
    let t = HelicalThread { pitch, depth, angle, clearance, from, to, left_handed: h.left_handed, start_angle: h.start_angle.unwrap_or(0.0) };
    Ok((body, cyl, t))
}

/// The groove of `t` on `cyl` as screw sweeps to cut, one per hand (`both`: a right- and a
/// left-handed groove crossing). Its section reaches a little past the face, so the cut leaves
/// no skin there.
pub fn helical_grooves(cyl: &kernel::CylinderFace, t: &HelicalThread, both: bool) -> Result<Vec<Body>> {
    let s = t.section()?;
    let turns = t.turns()?;
    let r = cyl.radius;
    if !(r.is_finite() && r > 1e-3 && t.from.is_finite() && t.to.is_finite()) {
        return Err(DocError::Invalid("thread placement".into()));
    }
    if !(turns >= 0.25) {
        return Err(DocError::Invalid(format!(
            "the threaded length is too short for a {:.3} mm pitch: it needs at least {:.3} mm",
            t.pitch,
            s.mouth + 0.25 * t.pitch
        )));
    }
    if turns > MAX_TURNS {
        return Err(DocError::Invalid(format!("a thread is limited to {MAX_TURNS:.0} turns")));
    }
    if !t.start_angle.is_finite() {
        return Err(DocError::Invalid("the start angle must be a number".into()));
    }
    // At most a pitch past either end of the face (a groove run off a free end).
    let over = t.pitch * (1.0 + 1e-9);
    if t.from < cyl.start.min(cyl.end) - over || t.to > cyl.start.max(cyl.end) + over {
        return Err(DocError::Invalid("the thread may run at most a pitch past the ends of its face".into()));
    }
    // Into the material: inwards for a shaft, outwards in a hole.
    let into = if cyl.internal { 1.0 } else { -1.0 };
    let r_root = r + into * s.deep;
    if !(r_root > r * 0.05) {
        return Err(DocError::Invalid(format!("a {:.3} mm deep thread does not fit a {:.3} mm shaft", s.deep, 2.0 * r)));
    }
    // Past the face, out of the material, the flanks go on a little: the section stays narrower
    // than the pitch, so the turns stay apart.
    let tan = (t.angle / 2.0).tan();
    let past = (0.05 * t.pitch).min((0.99 * t.pitch - s.mouth) / (2.0 * tan)).max(1e-4);
    let (r_air, half_air) = (r - into * past, s.mouth / 2.0 + past * tan);
    let section = [Vec2::new(r_root, -s.root / 2.0), Vec2::new(r_air, -half_air), Vec2::new(r_air, half_air), Vec2::new(r_root, s.root / 2.0)];
    let region = Region2 { outer: solidcore_record::Loop2::polygon(&section), holes: Vec::new() };
    let k = *cyl.axis;
    let u = k.cross(if k.x.abs() < 0.9 { Vec3::new(1.0, 0.0, 0.0) } else { Vec3::new(0.0, 1.0, 0.0) });
    let u = u.normalized().ok_or_else(|| DocError::Invalid("thread axis".into()))?;
    let x = u * t.start_angle.cos() + k.cross(u) * t.start_angle.sin();
    let base = cyl.axis_point + k * (t.from + s.mouth / 2.0);
    let plane = Plane { origin: base, x, y: k };
    let hands: &[bool] = if both {
        &[false, true]
    } else if t.left_handed {
        &[true]
    } else {
        &[false]
    };
    hands
        .iter()
        .map(|&left| {
            // A left-handed groove turns the same way about the reversed axis while rising along
            // it: the screw motion about −k rising −pitch per turn.
            let (axis, pitch) = if left { (-k, -t.pitch) } else { (k, t.pitch) };
            Ok(kernel::sweep_helix(&plane, &region, base, axis, pitch, turns)?)
        })
        .collect()
}

/// Evaluate a helical thread feature: its groove cut into the body.
pub(super) fn eval(vals: &BTreeMap<String, Value>, f: &Feature, st: &mut ModelState) -> Result<()> {
    let FeatureKind::Thread { face, designation, length, hand, pitch, depth, angle, clearance, start_offset, end_offset, start_angle, .. } = &f.kind
    else {
        return Err(DocError::Invalid("not a thread".into()));
    };
    let opt = |e: &Option<crate::expr::Expr>, k: Kind| -> Result<Option<f64>> { e.as_ref().map(|e| val(vals, e, k)).transpose() };
    let h = HelicalInputs {
        designation: designation.as_deref(),
        length: opt(length, Kind::Length)?,
        pitch: opt(pitch, Kind::Length)?,
        depth: opt(depth, Kind::Length)?,
        angle: opt(angle, Kind::Angle)?,
        clearance: opt(clearance, Kind::Length)?,
        start_offset: opt(start_offset, Kind::Length)?,
        end_offset: opt(end_offset, Kind::Length)?,
        left_handed: *hand == ThreadHand::Left,
        start_angle: opt(start_angle, Kind::Angle)?,
    };
    let (name, cyl, t) = helical_thread_on(st, face.0, &h)?;
    let grooves = helical_grooves(&cyl, &t, *hand == ThreadHand::Both)?;
    let tool = match grooves.as_slice() {
        [one] => one.clone(),
        many => kernel::union_all(&many.iter().collect::<Vec<_>>())?,
    };
    let i = st.bodies.iter().position(|b| b.name == name).ok_or_else(|| DocError::Invalid("threaded body".into()))?;
    let Some(mb) = st.bodies.get(i).cloned() else { return Err(DocError::Invalid("body".into())) };
    let input = mb.traced();
    let nb = kernel::boolean(&input.body, &tool, BoolOp::Cut)?.ok_or_else(|| DocError::Invalid("the thread removed everything".into()))?;
    // The groove's faces have no role to name them by.
    let groove = crate::lineage::Lineage::unnamed_faces(tool.face_count());
    let lineage = input.lineage.and_then(|l| crate::lineage::Lineage::step(&nb, &[&l, &groove]));
    if let Some(slot) = st.bodies.get_mut(i) {
        *slot = ModelBody::traced_new(mb.name, Traced { body: nb, lineage }, mb.feature);
    }
    Ok(())
}
