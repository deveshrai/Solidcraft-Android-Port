//! Gear tooth geometry: the outlines the gear commands (`cmd/gears.rs`) sketch and then extrude,
//! sweep or taper.
//!
//! External teeth are generated as a hob or rack cutter generates them: the basic rack of
//! ISO 53 (pressure angle, addendum, root clearance, the cutter's tip radius) rolls on the pitch
//! circle and the tooth is what it leaves. The flanks come out as involutes, the root as the
//! trochoid the rounded cutter tip traces, undercut included where few teeth make it. Profile
//! shift moves the cutter out from the pitch circle; backlash widens it. Internal teeth are
//! analytic involutes. A worm's section square to its axis has Archimedean flanks (a ZA worm,
//! straight-sided in the axial section). Bevel teeth use Tredgold's approximation: the virtual
//! spur gear on the back cone, projected from the pitch apex onto the heel plane.
//!
//! Everything here is plain geometry in millimetres and radians, without the document.

use solidcraft_geom::Vec2;
use solidcraft_io::Geom2;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

/// The basic rack and how the teeth are cut from it (coefficients are multiples of the module).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rack {
    /// Module (mm): the pitch diameter per tooth.
    pub m: f64,
    /// Pressure angle (radians).
    pub alpha: f64,
    /// Addendum coefficient (1 in ISO 53).
    pub ha: f64,
    /// Root clearance coefficient (0.25 in ISO 53).
    pub c: f64,
    /// The cutter's tip radius coefficient: the root fillet (0.38 in ISO 53 profile A).
    pub rho: f64,
    /// Profile shift coefficient.
    pub x: f64,
    /// How much thinner each tooth is at the pitch circle than the basic tooth (mm); half the
    /// backlash of a pair whose gears are thinned alike.
    pub thin: f64,
}

impl Rack {
    pub fn iso(m: f64, alpha: f64) -> Rack {
        Rack { m, alpha, ha: 1.0, c: 0.25, rho: 0.38, x: 0.0, thin: 0.0 }
    }
}

/// One generated tooth: its half-angle (from the tooth's centre line) at radii from the root to
/// the tip.
#[derive(Clone, Debug, PartialEq)]
pub struct Tooth {
    /// Teeth (a virtual gear's count may be fractional).
    pub z: f64,
    pub rf: f64,
    pub ra: f64,
    /// (radius, half-angle of the tooth), from `rf` up to `ra`.
    pub flank: Vec<(f64, f64)>,
    /// The tip came to a point below the asked tip circle and was cut back to a small land.
    pub pointed: bool,
    /// The cutter undercut the flank: it is thinner near the root than above it.
    pub undercut: bool,
}

/// Involute function.
pub fn inv(a: f64) -> f64 {
    a.tan() - a
}

/// Generate one external tooth of `z` teeth with tip radius `ra` (the pitch radius is `m z / 2`).
pub fn generate(z: f64, k: &Rack, ra: f64) -> Result<Tooth, String> {
    let m = k.m;
    if !(z.is_finite() && (3.0..=2000.0).contains(&z)) {
        return Err("a gear needs 3 to 2000 teeth".into());
    }
    if !(m.is_finite() && m > 1e-3 && m < 1e3) {
        return Err("the module must be between 0.001 and 1000 mm".into());
    }
    if !(k.alpha > 5f64.to_radians() && k.alpha < 45f64.to_radians()) {
        return Err("the pressure angle must be between 5° and 45°".into());
    }
    let r = m * z / 2.0;
    let (tan, cos) = (k.alpha.tan(), k.alpha.cos());
    // The cutter tooth in rack coordinates: u along the pitch line, eta out from the gear's
    // pitch circle. Its half-width at eta, its tip line, and the rounded tip corners.
    let hw = |eta: f64| PI * m / 4.0 + k.thin / 2.0 + (eta - k.x * m) * tan;
    let eta_t = k.x * m - (k.ha + k.c) * m;
    let rf = r + eta_t;
    if !(rf > 0.05 * r) {
        return Err("the root circle is at the centre: use more teeth or less profile shift".into());
    }
    if !(ra > rf + 0.05 * m) {
        return Err("the tip circle must be outside the root circle".into());
    }
    let w_t = hw(eta_t);
    if !(w_t > 1e-6 * m) {
        return Err("the cutter's tip comes to a point: lower the pressure angle or the root clearance".into());
    }
    let corner = FRAC_PI_4 - k.alpha / 2.0;
    let rho_c = (k.rho * m).clamp(0.0, 0.999 * w_t / corner.tan()).min(0.999 * (k.ha + k.c) * m * cos);
    let t_len = rho_c * corner.tan();
    let eta_top = (ra - r) + 2.0 * m;
    // The right half of the cutter's outline (u ≥ 0), from its centre on the tip line up the
    // flank; the left half mirrors it.
    let mut half: Vec<(f64, f64)> = Vec::new();
    let flat_end = w_t - t_len;
    for i in 0..=8 {
        half.push((flat_end * f64::from(i) / 8.0, eta_t));
    }
    if rho_c > 0.0 {
        let (cu, ce) = (flat_end, eta_t + rho_c);
        for i in 1..=24 {
            let a = -FRAC_PI_2 + (FRAC_PI_2 - k.alpha) * f64::from(i) / 24.0;
            half.push((cu + rho_c * a.cos(), ce + rho_c * a.sin()));
        }
    }
    let (fu, fe) = half.last().copied().unwrap_or((w_t, eta_t));
    let n_flank = 160;
    for i in 1..=n_flank {
        let eta = fe + (eta_top - fe) * f64::from(i) / f64::from(n_flank);
        half.push((fu + (eta - fe) * tan, eta));
    }
    let mut outline: Vec<(f64, f64)> = half.iter().rev().map(|(u, e)| (-u, *e)).collect();
    outline.extend(half.iter().skip(1).copied());

    // Target radii, packed towards the root where the trochoid runs nearly along the circle.
    let n_r = 240usize;
    let radii: Vec<f64> = (0..=n_r)
        .map(|j| {
            let t = j as f64 / n_r as f64;
            rf + (ra - rf) * (0.25 * t + 0.75 * t * t).max(1e-6)
        })
        .collect();
    let mut best = vec![f64::NEG_INFINITY; radii.len()];
    // Roll the cutter through every position where it reaches inside the tip circle.
    let reach = ((ra * ra - rf * rf).max(0.0).sqrt() + hw(eta_top).abs() + m) / r;
    let phi_max = reach.min(PI);
    let steps = ((2.0 * phi_max * r / (m / 400.0)).ceil() as usize).clamp(400, 40_000);
    let polar = |u: f64, e: f64, phi: f64| -> (f64, f64) {
        let (wx, wy) = (u - r * phi, r + e);
        let rr = wx.hypot(wy);
        let b = wx.atan2(wy) + phi;
        (rr * b.sin(), rr * b.cos())
    };
    for si in 0..=steps {
        let phi = -phi_max + 2.0 * phi_max * si as f64 / steps as f64;
        let mut prev: Option<(f64, f64)> = None;
        for (u, e) in &outline {
            let q = polar(*u, *e, phi);
            if let Some(p) = prev {
                cross(p, q, &radii, &mut best);
            }
            prev = Some(q);
        }
    }
    let half_pitch = PI / z;
    let mut flank = Vec::with_capacity(radii.len());
    for (rr, b) in radii.iter().zip(best.iter()) {
        if !b.is_finite() {
            return Err("the cutter never reaches this radius (internal error)".into());
        }
        flank.push((*rr, half_pitch - b));
    }
    // A tooth that comes to a point below the tip circle is cut back to a land of 0.05 m.
    let land = |rr: f64| 0.025 * m / rr;
    let mut pointed = false;
    if let Some(i) = flank.iter().position(|(rr, a)| *a < land(*rr)) {
        if i < 4 {
            return Err("the teeth are cut away entirely: use more teeth, a smaller pressure angle or more profile shift".into());
        }
        pointed = true;
        let (r0, a0) = flank.get(i - 1).copied().unwrap_or((rf, 0.0));
        let (r1, a1) = flank.get(i).copied().unwrap_or((ra, 0.0));
        let (l0, l1) = (a0 - land(r0), a1 - land(r1));
        let t = if (l0 - l1).abs() > 1e-15 { l0 / (l0 - l1) } else { 0.0 };
        let rt = r0 + (r1 - r0) * t.clamp(0.0, 1.0);
        flank.truncate(i);
        flank.push((rt, land(rt)));
    }
    let tip = flank.last().map_or(ra, |f| f.0);
    if !(flank.first().is_some_and(|(_, a)| *a > 0.0) && flank.first().is_some_and(|(_, a)| *a < half_pitch)) {
        return Err("the teeth leave no root between them: use fewer teeth or less profile shift".into());
    }
    // Undercut: the cutter's tip line passes below the interference point (fewer than
    // 2 (ha − x) / sin²α teeth: 17 at 20° without shift).
    let undercut = z * k.alpha.sin().powi(2) / 2.0 < k.ha - k.x - 1e-9;
    Ok(Tooth { z, rf, ra: tip, flank, pointed, undercut })
}

/// Record, for each target radius the segment p–q crosses, the angle (from +y towards +x) at
/// which it does, if it's the largest so far.
fn cross(p: (f64, f64), q: (f64, f64), radii: &[f64], best: &mut [f64]) {
    let (rp, rq) = (p.0.hypot(p.1), q.0.hypot(q.1));
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let a = dx * dx + dy * dy;
    if a < 1e-30 {
        return;
    }
    // The segment's nearest point to the centre may lie between its ends (where it runs along
    // the root circle).
    let t_near = (-(p.0 * dx + p.1 * dy) / a).clamp(0.0, 1.0);
    let lo = (p.0 + t_near * dx).hypot(p.1 + t_near * dy);
    let hi = rp.max(rq);
    let start = radii.partition_point(|r| *r < lo);
    let end = radii.partition_point(|r| *r <= hi);
    for j in start..end {
        let Some(&rt) = radii.get(j) else { continue };
        // |p + t d| = rt.
        let b = 2.0 * (p.0 * dx + p.1 * dy);
        let c = p.0 * p.0 + p.1 * p.1 - rt * rt;
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            continue;
        }
        let sq = disc.sqrt();
        for t in [(-b - sq) / (2.0 * a), (-b + sq) / (2.0 * a)] {
            if (-1e-12..=1.0 + 1e-12).contains(&t) {
                let (x, y) = (p.0 + t * dx, p.1 + t * dy);
                let ang = x.atan2(y);
                if let Some(bj) = best.get_mut(j)
                    && ang > *bj
                {
                    *bj = ang;
                }
            }
        }
    }
}

/// Internal teeth (a ring gear), analytic involutes. An internal gear's space is shaped like an
/// external tooth, so the [`Tooth`] returned describes the spaces: their half-angle at radii
/// from the tip circle (inside) out to the root circle.
pub fn internal(z: f64, k: &Rack) -> Result<Tooth, String> {
    let m = k.m;
    if !(z.is_finite() && (6.0..=2000.0).contains(&z)) {
        return Err("an internal gear needs 6 to 2000 teeth".into());
    }
    if !(m.is_finite() && m > 1e-3 && m < 1e3) {
        return Err("the module must be between 0.001 and 1000 mm".into());
    }
    if !(k.alpha > 5f64.to_radians() && k.alpha < 45f64.to_radians()) {
        return Err("the pressure angle must be between 5° and 45°".into());
    }
    let r = m * z / 2.0;
    let rb = r * k.alpha.cos();
    let ra = r - (k.ha - k.x) * m;
    let rf = r + (k.ha + k.c + k.x) * m;
    if !(ra > 0.2 * r) {
        return Err("the internal teeth reach the centre: use more teeth".into());
    }
    // The space at the pitch circle (wider for backlash and a positive shift).
    let e = PI * m / 2.0 + 2.0 * k.x * m * k.alpha.tan() + k.thin;
    let space = |rr: f64| {
        let ar = if rr > rb { (rb / rr).acos() } else { 0.0 };
        e / (2.0 * r) + inv(k.alpha) - inv(ar)
    };
    let half_pitch = PI / z;
    let n = 120;
    let mut flank = Vec::new();
    for i in 0..=n {
        let rr = ra + (rf - ra) * f64::from(i) / f64::from(n);
        flank.push((rr, space(rr)));
    }
    if flank.iter().any(|(rr, a)| half_pitch - a < 0.025 * m / rr) {
        return Err("the internal teeth come to a point: use fewer teeth on the mate, more teeth here or a smaller pressure angle".into());
    }
    if flank.last().is_some_and(|(_, a)| *a <= 0.0) {
        return Err("the internal teeth leave no space between them".into());
    }
    Ok(Tooth { z, rf: ra, ra: rf, flank, pointed: false, undercut: false })
}

fn polar(r: f64, a: f64) -> Vec2 {
    Vec2::new(r * a.cos(), r * a.sin())
}

/// About `n` points of a flank, spaced along its length so the spline through them follows it.
fn resample(pts: &[Vec2], n: usize) -> Vec<Vec2> {
    if pts.len() <= n {
        return pts.to_vec();
    }
    let mut acc = vec![0.0];
    for w in pts.windows(2) {
        if let [a, b] = w {
            acc.push(acc.last().copied().unwrap_or(0.0) + a.dist(*b));
        }
    }
    let total = acc.last().copied().unwrap_or(0.0);
    if !(total > 0.0) {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity(n + 1);
    let mut j = 0;
    for i in 0..=n {
        let s = total * i as f64 / n as f64;
        while j + 1 < acc.len() - 1 && acc.get(j + 1).is_some_and(|x| *x < s) {
            j += 1;
        }
        let (Some(&s0), Some(&s1), Some(&p0), Some(&p1)) = (acc.get(j), acc.get(j + 1), pts.get(j), pts.get(j + 1)) else { continue };
        let t = if s1 > s0 { ((s - s0) / (s1 - s0)).clamp(0.0, 1.0) } else { 0.0 };
        out.push(p0 + (p1 - p0) * t);
    }
    out.dedup_by(|a, b| a.dist(*b) < 1e-9);
    out
}

/// Points of a tooth outline, mapped by `map` from (radius, angle) to the plane.
type Map<'a> = &'a dyn Fn(f64, f64) -> Vec2;

/// The closed outline of an external gear (or the hole of an internal one) of `count` teeth
/// spaced `2π / count`, tooth 0 centred on +x, each tooth's flanks a spline. `map` places a
/// point given in polar coordinates (a bevel gear's projection); `None` is plain polar. The
/// tooth's half-angles are multiplied by `angle_scale` (a bevel gear's virtual angles).
pub fn outline(t: &Tooth, count: usize, angle_scale: f64, map: Option<Map>) -> Vec<Geom2> {
    let place = |r: f64, a: f64| match map {
        Some(f) => f(r, a),
        None => polar(r, a),
    };
    let pitch = TAU / count as f64;
    // A flank's Bézier follows its points to a thousandth of the tooth's height (the points
    // themselves are that close to the true involute).
    let tol = 1e-3 * (t.ra - t.rf).abs().max(1e-6);
    // Every tooth's flanks have tooth 0's shape: plan their pieces once.
    let right0: Vec<Vec2> = t.flank.iter().map(|(r, a)| place(*r, -a * angle_scale)).collect();
    let left0: Vec<Vec2> = t.flank.iter().rev().map(|(r, a)| place(*r, a * angle_scale)).collect();
    let (right_plan, left_plan) = (flank_plan(&right0, tol, 3), flank_plan(&left0, tol, 3));
    let mut out = Vec::new();
    let (inner_r, outer_r) = (t.flank.first().map_or(t.rf, |f| f.0), t.flank.last().map_or(t.ra, |f| f.0));
    let (inner_half, outer_half) = (t.flank.first().map_or(0.0, |f| f.1) * angle_scale, t.flank.last().map_or(0.0, |f| f.1) * angle_scale);
    for i in 0..count {
        let c = pitch * i as f64;
        // Up the right flank (clockwise side), across the far end, down the left flank, then
        // along the near circle to the next tooth.
        let right: Vec<Vec2> = t.flank.iter().map(|(r, a)| place(*r, c - a * angle_scale)).collect();
        let left: Vec<Vec2> = t.flank.iter().rev().map(|(r, a)| place(*r, c + a * angle_scale)).collect();
        out.extend(flank_curves(&right, right_plan.as_deref()));
        if outer_half > 1e-9 {
            out.push(arc(c - outer_half, c + outer_half, &place, outer_r));
        }
        out.extend(flank_curves(&left, left_plan.as_deref()));
        let (a0, a1) = (c + inner_half, c + pitch - inner_half);
        if a1 - a0 > 1e-9 {
            out.push(arc(a0, a1, &place, inner_r));
        }
    }
    out
}

/// A flank as few curves as follow it: one Bézier when one does (degree 3 to 9), else two or a
/// few end to end, else a spline through its points. Every curve becomes a face of the gear, and
/// every face makes each boolean on it slower: a spline through points makes a face for each
/// span between them. `plan` is [`flank_plan`]'s for a flank of the same shape (every tooth's).
fn flank_curves(pts: &[Vec2], plan: Option<&[Piece]>) -> Vec<Geom2> {
    let curves: Option<Vec<Geom2>> = plan.and_then(|plan| {
        plan.iter()
            .map(|&(a, b, n)| {
                let ctrl = fit_bezier(pts.get(a..=b)?, n)?.0;
                Some(Geom2::Spline { pts: ctrl, control: true, degree: n as u8 })
            })
            .collect()
    });
    curves.unwrap_or_else(|| vec![Geom2::Spline { pts: resample(pts, 8), control: false, degree: 3 }])
}

/// A Bézier piece of a flank: its first and last point's index, and its degree.
type Piece = (usize, usize, usize);

/// Highest degree of a flank's Bézier: the sketch's highest exact one.
const MAX_FLANK_DEGREE: usize = solidcraft_geom::MAX_BEZIER_DEGREE;

/// The lowest degree whose Bézier follows `pts` to within `tol`.
fn lowest_degree(pts: &[Vec2], tol: f64) -> Option<usize> {
    (3..=MAX_FLANK_DEGREE).take_while(|n| pts.len() >= n + 3).find(|&n| fit_bezier(pts, n).is_some_and(|(_, err)| err <= tol))
}

/// The pieces that follow a flank's points to within `tol`: one if one does, else the two that
/// split it where their degrees add up lowest (the root fillet and the involute), else halves of
/// halves, at most `depth` times. None when that isn't enough.
fn flank_plan(pts: &[Vec2], tol: f64, depth: u32) -> Option<Vec<Piece>> {
    plan_from(pts, 0, tol, depth)
}

fn plan_from(pts: &[Vec2], at: usize, tol: f64, depth: u32) -> Option<Vec<Piece>> {
    let last = pts.len().checked_sub(1)?;
    if let Some(n) = lowest_degree(pts, tol) {
        return Some(vec![(at, at + last, n)]);
    }
    if depth == 0 || pts.len() < 12 {
        return None;
    }
    let step = (pts.len() / 24).max(1);
    let best = (6..last.saturating_sub(5))
        .step_by(step)
        .filter_map(|k| {
            let (a, b) = (lowest_degree(pts.get(..=k)?, tol)?, lowest_degree(pts.get(k..)?, tol)?);
            Some((a + b, k, a, b))
        })
        .min_by_key(|x| x.0);
    if let Some((_, k, a, b)) = best {
        return Some(vec![(at, at + k, a), (at + k, at + last, b)]);
    }
    let mid = pts.len() / 2;
    let mut out = plan_from(pts.get(..=mid)?, at, tol, depth - 1)?;
    out.extend(plan_from(pts.get(mid..)?, at + mid, tol, depth - 1)?);
    Some(out)
}

/// The degree-`n` Bézier closest to `pts` (least squares, its ends on the first and last point),
/// and how far the farthest point is from it.
fn fit_bezier(pts: &[Vec2], n: usize) -> Option<(Vec<Vec2>, f64)> {
    let (first, last) = (*pts.first()?, *pts.last()?);
    // Chord-length parameters, then each point's nearest parameter on the last fit.
    let mut t = vec![0.0];
    for w in pts.windows(2) {
        if let [a, b] = w {
            t.push(t.last().copied().unwrap_or(0.0) + a.dist(*b));
        }
    }
    let total = t.last().copied().unwrap_or(0.0);
    if !(total > 0.0) {
        return None;
    }
    t.iter_mut().for_each(|x| *x /= total);
    let mut ctrl = Vec::new();
    for _ in 0..6 {
        ctrl = least_squares(pts, &t, n, first, last)?;
        for (ti, q) in t.iter_mut().zip(pts) {
            for _ in 0..2 {
                let (p, d1, d2) = bezier_eval(&ctrl, *ti);
                let r = p - *q;
                let den = d1.dot(d1) + r.dot(d2);
                if den.abs() > 1e-300 {
                    *ti = (*ti - r.dot(d1) / den).clamp(0.0, 1.0);
                }
            }
        }
    }
    let err = t.iter().zip(pts).map(|(ti, q)| bezier_eval(&ctrl, *ti).0.dist(*q)).fold(0.0, f64::max);
    err.is_finite().then_some((ctrl, err))
}

/// Least-squares inner control points of a degree-`n` Bézier through `first` and `last`, point
/// `pts[i]` at parameter `t[i]`.
fn least_squares(pts: &[Vec2], t: &[f64], n: usize, first: Vec2, last: Vec2) -> Option<Vec<Vec2>> {
    let m = n - 1;
    let mut a = vec![vec![0.0; m]; m];
    let mut rhs = vec![Vec2::ZERO; m];
    for (ti, q) in t.iter().zip(pts) {
        let b = bernstein(n, *ti);
        let (b0, bn) = (b.first().copied().unwrap_or(0.0), b.last().copied().unwrap_or(0.0));
        let r = *q - first * b0 - last * bn;
        for j in 0..m {
            let bj = b.get(j + 1).copied().unwrap_or(0.0);
            if let Some(x) = rhs.get_mut(j) {
                *x += r * bj;
            }
            for k in 0..m {
                if let Some(x) = a.get_mut(j).and_then(|row| row.get_mut(k)) {
                    *x += bj * b.get(k + 1).copied().unwrap_or(0.0);
                }
            }
        }
    }
    // Gaussian elimination with partial pivoting.
    for col in 0..m {
        let piv = (col..m).max_by(|&i, &j| {
            let (x, y) = (a.get(i).and_then(|r| r.get(col)), a.get(j).and_then(|r| r.get(col)));
            x.map(|v| v.abs()).partial_cmp(&y.map(|v| v.abs())).unwrap_or(std::cmp::Ordering::Equal)
        })?;
        a.swap(col, piv);
        rhs.swap(col, piv);
        let d = *a.get(col)?.get(col)?;
        if !(d.abs() > 1e-14) {
            return None;
        }
        let (pivot_row, pivot_rhs) = (a.get(col)?.clone(), *rhs.get(col)?);
        for row in col + 1..m {
            let f = a.get(row)?.get(col)? / d;
            for (x, p) in a.get_mut(row)?.iter_mut().zip(&pivot_row) {
                *x -= f * p;
            }
            let r = rhs.get_mut(row)?;
            *r -= pivot_rhs * f;
        }
    }
    let mut x = vec![Vec2::ZERO; m];
    for row in (0..m).rev() {
        let mut s = *rhs.get(row)?;
        for k in row + 1..m {
            s -= *x.get(k)? * *a.get(row)?.get(k)?;
        }
        *x.get_mut(row)? = s * (1.0 / a.get(row)?.get(row)?);
    }
    let mut ctrl = Vec::with_capacity(n + 1);
    ctrl.push(first);
    ctrl.extend(x);
    ctrl.push(last);
    ctrl.iter().all(|p| p.is_finite()).then_some(ctrl)
}

/// The `n + 1` Bernstein polynomials of degree `n` at `t`.
fn bernstein(n: usize, t: f64) -> Vec<f64> {
    let mut b = vec![0.0; n + 1];
    if let Some(x) = b.first_mut() {
        *x = 1.0;
    }
    for k in 1..=n {
        for j in (0..=k).rev() {
            let prev = if j > 0 { b.get(j - 1).copied().unwrap_or(0.0) } else { 0.0 };
            let cur = b.get(j).copied().unwrap_or(0.0);
            if let Some(x) = b.get_mut(j) {
                *x = cur * (1.0 - t) + prev * t;
            }
        }
    }
    b
}

/// A Bézier's point and first two derivatives at `t`.
fn bezier_eval(ctrl: &[Vec2], t: f64) -> (Vec2, Vec2, Vec2) {
    let n = ctrl.len().saturating_sub(1);
    let point = |c: &[Vec2], deg: usize| -> Vec2 { bernstein(deg, t).iter().zip(c).fold(Vec2::ZERO, |s, (b, p)| s + *p * *b) };
    let diff = |c: &[Vec2], k: usize| -> Vec<Vec2> { c.iter().zip(c.iter().skip(1)).map(|(a, b)| (*b - *a) * k as f64).collect() };
    let d1 = diff(ctrl, n);
    let d2 = diff(&d1, n.saturating_sub(1));
    (point(ctrl, n), if n >= 1 { point(&d1, n - 1) } else { Vec2::ZERO }, if n >= 2 { point(&d2, n - 2) } else { Vec2::ZERO })
}

/// A counter-clockwise arc about the origin between two angles at a tooth radius, its ends
/// exactly where `place` puts the flank ends.
fn arc(a0: f64, a1: f64, place: &dyn Fn(f64, f64) -> Vec2, r_tooth: f64) -> Geom2 {
    Geom2::Arc { c: Vec2::new(0.0, 0.0), a: place(r_tooth, a0), b: place(r_tooth, a1) }
}

/// A straight rack: `teeth` teeth along +x from x = 0, the pitch line on y = 0, teeth up to
/// `ha m`, the back at `-(ha + c) m - back`. The outline starts and ends mid-space.
pub fn rack(k: &Rack, teeth: usize, back: f64) -> Result<Vec<Geom2>, String> {
    let m = k.m;
    if !(m.is_finite() && m > 1e-3 && m < 1e3) {
        return Err("the module must be between 0.001 and 1000 mm".into());
    }
    if !(1..=2000).contains(&teeth) {
        return Err("a rack needs 1 to 2000 teeth".into());
    }
    if !(back.is_finite() && back > 0.0) {
        return Err("the rack's back must be thicker than zero".into());
    }
    let p = PI * m;
    let tan = k.alpha.tan();
    let (y_tip, y_root) = ((k.ha + k.x) * m, -(k.ha + k.c - k.x) * m);
    // Tooth half-width at height y.
    let hw = |y: f64| PI * m / 4.0 - k.thin / 2.0 - (y - k.x * m) * tan;
    let (w_tip, w_root) = (hw(y_tip), hw(y_root));
    if !(w_tip > 0.0) {
        return Err("the rack's teeth come to a point: lower the pressure angle or the addendum".into());
    }
    if !(w_root < p / 2.0) {
        return Err("the rack's teeth leave no root between them".into());
    }
    let corner = FRAC_PI_4 - k.alpha / 2.0;
    let gap = p / 2.0 - w_root;
    let rho = (k.rho * m).clamp(0.0, 0.999 * gap / corner.tan()).min(0.999 * (y_tip - y_root) * k.alpha.cos());
    let t_len = rho * corner.tan();
    let v = |x: f64, y: f64| Vec2::new(x, y);
    let mut out = Vec::new();
    let x0 = -p / 2.0;
    let x_end = (teeth as f64 - 0.5) * p;
    let y_back = y_root - back;
    let mut cur = v(x0, y_root);
    let dir = (k.alpha.sin(), k.alpha.cos());
    for i in 0..teeth {
        let c = p * i as f64;
        // Root to the left of tooth i, its fillet, up the left flank, the tip, down the right
        // flank, its fillet.
        let (lx, rx) = (c - w_root, c + w_root);
        let fl_a = v(lx - t_len, y_root);
        if fl_a.x - cur.x > 1e-9 {
            out.push(Geom2::Line(cur, fl_a));
        }
        let fl_b = v(lx + t_len * dir.0, y_root + t_len * dir.1);
        if rho > 0.0 {
            // Centre above the root line, inside the space: counter-clockwise from the root up
            // to the flank.
            let cen = v(lx - t_len, y_root + rho);
            out.push(Geom2::Arc { c: cen, a: fl_a, b: fl_b });
        }
        let tl = v(c - w_tip, y_tip);
        let tr = v(c + w_tip, y_tip);
        out.push(Geom2::Line(if rho > 0.0 { fl_b } else { v(lx, y_root) }, tl));
        out.push(Geom2::Line(tl, tr));
        let fr_a = v(rx - t_len * dir.0, y_root + t_len * dir.1);
        let fr_b = v(rx + t_len, y_root);
        out.push(Geom2::Line(tr, if rho > 0.0 { fr_a } else { v(rx, y_root) }));
        if rho > 0.0 {
            let cen = v(rx + t_len, y_root + rho);
            out.push(Geom2::Arc { c: cen, a: fr_a, b: fr_b });
        }
        cur = if rho > 0.0 { fr_b } else { v(rx, y_root) };
    }
    let end = v(x_end, y_root);
    if end.x - cur.x > 1e-9 {
        out.push(Geom2::Line(cur, end));
    }
    out.push(Geom2::Line(end, v(x_end, y_back)));
    out.push(Geom2::Line(v(x_end, y_back), v(x0, y_back)));
    out.push(Geom2::Line(v(x0, y_back), v(x0, y_root)));
    Ok(out)
}

/// A worm's section square to its axis (ZA worm): `starts` lobes with Archimedean flanks.
/// `r1` is the pitch radius, `lead` the axial advance per turn.
pub fn worm_section(k: &Rack, r1: f64, starts: usize, lead: f64) -> Result<(Vec<Geom2>, f64, f64), String> {
    let m = k.m;
    if !(1..=12).contains(&starts) {
        return Err("a worm has 1 to 12 starts".into());
    }
    let px = PI * m;
    let ra = r1 + k.ha * m;
    let rf = r1 - (k.ha + k.c) * m;
    if !(rf > 0.1 * r1) {
        return Err("the worm's root reaches its axis: use a larger worm diameter".into());
    }
    let tan = k.alpha.tan();
    // Axial half-thickness at radius r, and its angle in the section.
    let t = |r: f64| px / 4.0 - k.thin / 2.0 - (r - r1) * tan;
    if !(t(ra) > 0.0) {
        return Err("the worm's thread comes to a point: lower the pressure angle".into());
    }
    let ang = |r: f64| TAU * t(r) / lead;
    let half = PI / starts as f64;
    if !(ang(rf) < half) {
        return Err("the worm's threads leave no root between them".into());
    }
    // The flank is an Archimedean spiral (its angle linear in the radius), sampled for its Bézier.
    let flank: Vec<(f64, f64)> = (0..=32).map(|i| rf + (ra - rf) * f64::from(i) / 32.0).map(|r| (r, ang(r))).collect();
    let tooth = Tooth { z: starts as f64, rf, ra, flank, pointed: false, undercut: false };
    Ok((outline(&tooth, starts, 1.0, None), ra, rf))
}

/// A bevel gear's heel outline on the plane z = 0, the pitch apex at z = `apex`: the virtual
/// gear's tooth on the back cone, projected from the apex. `delta` is the pitch cone angle,
/// `big_r` the pitch radius at the heel.
pub fn bevel_outline(t: &Tooth, z: usize, delta: f64, big_r: f64) -> Vec<Geom2> {
    let (sd, cd) = (delta.sin(), delta.cos());
    let apex = big_r / delta.tan();
    let back_apex = -big_r * delta.tan();
    // A point at distance rv from the back cone's apex (on the virtual gear), at azimuth phi.
    let map = move |rv: f64, phi: f64| {
        let qr = rv * cd;
        let qz = back_apex + rv * sd;
        let s = apex / (apex - qz);
        polar(qr * s, phi)
    };
    outline(t, z, 1.0 / cd, Some(&map))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deg(d: f64) -> f64 {
        d.to_radians()
    }

    /// Above the base circle a generated flank is the involute: the tooth's half-angle at radius
    /// r is s / 2r + inv α − inv α_r, s the tooth thickness at the pitch circle.
    #[test]
    fn generated_flanks_are_involutes() {
        for (z, x) in [(20.0, 0.0), (40.0, 0.3), (12.0, 0.5), (60.0, -0.2)] {
            let k = Rack { x, ..Rack::iso(2.0, deg(20.0)) };
            let r = z;
            let ra = r + (1.0 + x) * 2.0;
            let t = generate(z, &k, ra).unwrap_or_else(|e| panic!("{z}: {e}"));
            let rb = r * deg(20.0).cos();
            let s = PI * 2.0 / 2.0 + 2.0 * x * 2.0 * deg(20.0).tan();
            // The involute starts at the form circle, where the cutter's straight flank ends
            // (above its rounded tip) meets the line of action.
            let eta_f = (x - 1.25) * 2.0 + 0.38 * 2.0 * (1.0 - deg(20.0).sin());
            let form = rb.hypot(r * deg(20.0).sin() + eta_f / deg(20.0).sin());
            for (rr, a) in t.flank.iter().filter(|(rr, _)| *rr > form + 0.02 && *rr < t.ra - 1e-6) {
                let exact = s / (2.0 * r) + inv(deg(20.0)) - inv((rb / rr).acos());
                assert!((a - exact).abs() * rr < 1e-3, "z {z} x {x} r {rr}: {a} vs {exact}");
            }
            assert!((t.rf - (r - (1.25 - x) * 2.0)).abs() < 1e-9);
            assert!(!t.pointed, "{z}");
        }
    }

    /// Few teeth without profile shift are undercut (fewer than 17 at 20°); shifted they are not.
    #[test]
    fn undercut_shows_below_seventeen_teeth() {
        let k = Rack::iso(1.0, deg(20.0));
        assert!(generate(8.0, &k, 5.0).map(|t| t.undercut).unwrap_or(false));
        assert!(!generate(30.0, &k, 16.0).map(|t| t.undercut).unwrap_or(true));
        let shifted = Rack { x: 0.6, ..k };
        assert!(!generate(8.0, &shifted, 4.0 + 1.6).map(|t| t.undercut).unwrap_or(true));
    }

    /// A heavily shifted small gear's teeth come to a point and are cut back.
    #[test]
    fn pointed_teeth_are_cut_back() {
        let k = Rack { x: 1.2, ..Rack::iso(1.0, deg(20.0)) };
        let t = generate(8.0, &k, 4.0 + 2.2).unwrap_or_else(|e| panic!("{e}"));
        assert!(t.pointed && t.ra < 6.2);
    }

    #[test]
    fn bad_input_is_refused() {
        let k = Rack::iso(1.0, deg(20.0));
        assert!(generate(f64::NAN, &k, 10.0).is_err());
        assert!(generate(2.0, &k, 10.0).is_err());
        assert!(generate(20.0, &Rack { m: 0.0, ..k }, 10.0).is_err());
        assert!(generate(20.0, &Rack { alpha: deg(60.0), ..k }, 11.0).is_err());
        assert!(rack(&k, 0, 2.0).is_err());
        assert!(worm_section(&k, 10.0, 0, PI).is_err());
        assert!(internal(3.0, &k).is_err());
    }

    /// An outline is closed: every curve starts where the previous one ended.
    #[test]
    fn outlines_close() {
        fn ends(g: &Geom2) -> (Vec2, Vec2) {
            match g {
                Geom2::Line(a, b) => (*a, *b),
                Geom2::Arc { a, b, .. } => (*a, *b),
                Geom2::Spline { pts, .. } => (pts.first().copied().unwrap_or_default(), pts.last().copied().unwrap_or_default()),
                _ => (Vec2::ZERO, Vec2::ZERO),
            }
        }
        let k = Rack::iso(1.5, deg(20.0));
        let t = generate(17.0, &k, 8.5 * 1.5 + 1.5).unwrap_or_else(|e| panic!("{e}"));
        let mut all = vec![outline(&t, 17, 1.0, None), rack(&k, 6, 3.0).unwrap_or_default()];
        all.push(worm_section(&k, 12.0, 2, 2.0 * PI * 1.5).map(|w| w.0).unwrap_or_default());
        all.push(outline(&internal(40.0, &k).unwrap_or_else(|e| panic!("{e}")), 40, 1.0, None));
        for g in &all {
            assert!(g.len() > 4);
            for w in g.windows(2) {
                if let [a, b] = w {
                    assert!(ends(a).1.dist(ends(b).0) < 1e-6, "{a:?} → {b:?}");
                }
            }
            let (first, last) = (g.first().map(ends), g.last().map(ends));
            if let (Some(f), Some(l)) = (first, last) {
                assert!(f.0.dist(l.1) < 1e-6);
            }
        }
    }

    /// A flank is at most two Béziers (its root fillet and its involute; one face of the gear
    /// each, not one a span; three with an undercut), and each follows the generated flank to a thousandth of the tooth
    /// height.
    #[test]
    fn a_flank_is_two_beziers_on_the_generated_points() {
        for (z, x) in [(18.0, 0.0), (8.0, 0.0), (40.0, 0.3), (12.0, 0.5)] {
            let k = Rack { x, ..Rack::iso(1.5, deg(20.0)) };
            let t = generate(z, &k, 1.5 * z / 2.0 + (1.0 + x) * 1.5).unwrap_or_else(|e| panic!("{z}: {e}"));
            let o = outline(&t, z as usize, 1.0, None);
            let splines: Vec<_> = o.iter().filter(|g| matches!(g, Geom2::Spline { .. })).collect();
            // An undercut tooth's flank has a corner between its fillet and its involute.
            let most = if t.undercut { 3 } else { 2 };
            assert!(splines.len() <= 2 * most * z as usize, "z {z} x {x}: {} flank curves", splines.len());
            let flank: Vec<Vec2> = t.flank.iter().map(|(r, a)| polar(*r, -a)).collect();
            let pieces = flank_plan(&flank, 1e-3 * (t.ra - t.rf), 3).unwrap_or_else(|| panic!("z {z} x {x}: no fit"));
            assert!(pieces.len() <= most, "z {z} x {x}: {} pieces", pieces.len());
            for &(a, b, n) in &pieces {
                let err = flank.get(a..=b).and_then(|p| fit_bezier(p, n)).map_or(f64::MAX, |f| f.1);
                assert!(err <= 1e-3 * (t.ra - t.rf), "z {z} x {x}: {err}");
            }
            for g in splines {
                let Geom2::Spline { pts, control: true, degree } = g else { panic!("z {z}: not a Bézier") };
                assert_eq!(pts.len(), *degree as usize + 1);
            }
        }
        let k = Rack::iso(1.5, deg(20.0));
        let (worm, _, _) = worm_section(&k, 7.5, 3, 3.0 * PI * 1.5).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(worm.iter().filter(|g| matches!(g, Geom2::Spline { control: true, .. })).count(), 6);
    }
}
