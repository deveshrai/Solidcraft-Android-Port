//! A boolean candidate whose edges run far outside its operands is discarded before it is
//! meshed (#3020).
//!
//! The retries of a boolean mesh every candidate to measure its volume. A candidate made at a
//! coarse tolerance can come out with an edge that leaves the body altogether: in the design of
//! #3020, a face of a revolved dovetail (a cone) got a trim loop several metres long on a part
//! 155 mm across, its points kilometres from the cone, and meshing that face never finished.
//! Its volume (±1e10 mm³) would have failed the check anyway; only measuring it hung.
//!
//! Every edge of a union, a cut or an intersection lies on the operands' faces, within the
//! boolean's tolerance (a small fraction of their size). So a candidate with a point of an edge
//! further outside the box the operands span than [`MARGIN`] of their size is not a result of
//! the operation, whatever its faces are, and it is discarded without meshing it. That needs no
//! search for the edge's points on its faces' surfaces, which is the very thing that runs away
//! on such a face.

use std::collections::HashSet;

use solidcraft_geom::{Aabb3, Mesh, Vec3};
use truck_modeling as mt;

use crate::body::{Body, from_p3};

#[cfg(test)]
#[path = "candidate_reach_tests.rs"]
mod tests;

/// Samples along each edge, ends included: a runaway edge leaves the body over a good part of
/// its length (the loop of #3020 was thousands of times the body's size).
const EDGE_SAMPLES: u32 = 16;

/// How far an edge of a candidate may run outside the box of the operands' meshes, as a fraction
/// of the operands' size: well above any boolean tolerance (2e-3 of the size at the coarsest) and
/// the temporary face shifts of the retries (1.4e-2 of it), well below the metres a runaway edge
/// spans.
pub(crate) const MARGIN: f64 = 0.05;

/// Where a candidate's edges may run when the operands could not be meshed: their edges only
/// bound them loosely (a sphere's seam spans half of it), so the box of those grows by their
/// whole size.
const MARGIN_UNMESHED: f64 = 1.0;

/// The box the operands of a boolean span, grown by the room a result's edges may take.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reach {
    bounds: Aabb3,
    margin: f64,
}

impl Reach {
    /// The reach of operands `a` and `b` of size `size`, from their meshes where both were made
    /// (`meshes`), else from their edges. `None` when neither gives a finite box.
    pub(crate) fn new(a: &Body, b: &Body, meshes: (Option<&Mesh>, Option<&Mesh>), size: f64) -> Option<Reach> {
        let mut bounds = Aabb3::EMPTY;
        let margin = match meshes {
            (Some(ma), Some(mb)) => {
                ma.positions.iter().chain(&mb.positions).for_each(|p| bounds.add(*p));
                MARGIN * size
            }
            _ => {
                for body in [a, b] {
                    edge_points(body, |p| {
                        bounds.add(p);
                        true
                    })?;
                }
                MARGIN_UNMESHED * size
            }
        };
        (!bounds.is_empty() && margin.is_finite() && margin > 0.0).then_some(Reach { bounds, margin })
    }

    /// Why `candidate` cannot be a result of a boolean of these operands: an edge of it runs
    /// further outside them than the margin, or has a point that is not finite. `None` when its
    /// edges stay within reach, and when they could not be sampled (it is then meshed and
    /// checked as before).
    pub(crate) fn runaway(&self, candidate: &Body) -> Option<String> {
        let (lo, hi) = (self.bounds.min, self.bounds.max);
        let mut worst = 0.0f64;
        let mut infinite = false;
        edge_points(candidate, |p| {
            if !p.is_finite() {
                infinite = true;
                return false;
            }
            let outside = Vec3::new(excess(p.x, lo.x, hi.x), excess(p.y, lo.y, hi.y), excess(p.z, lo.z, hi.z)).len();
            worst = worst.max(outside);
            true
        })?;
        if infinite {
            Some("a result whose edge has points that are not finite".into())
        } else if worst > self.margin {
            Some(format!("a result whose edge runs {worst:.4e} mm outside the operands (more than {:.4e} mm), not meshed to measure it", self.margin))
        } else {
            None
        }
    }
}

/// How far `x` lies outside `[lo, hi]`.
fn excess(x: f64, lo: f64, hi: f64) -> f64 {
    (lo - x).max(x - hi).max(0.0)
}

/// Visits [`EDGE_SAMPLES`] points along each edge of `body` (once per edge) until `visit` says
/// stop. `None` when the curves could not be evaluated.
fn edge_points(body: &Body, mut visit: impl FnMut(Vec3) -> bool) -> Option<()> {
    crate::guard("candidate edges", || {
        use mt::{BoundedCurve, ParametricCurve};
        let mut seen = HashSet::new();
        for edge in body.solid.edge_iter() {
            if !seen.insert(edge.id()) {
                continue;
            }
            let curve = edge.curve();
            let (t0, t1) = curve.range_tuple();
            for i in 0..=EDGE_SAMPLES {
                let t = t0 + (t1 - t0) * f64::from(i) / f64::from(EDGE_SAMPLES);
                if !visit(from_p3(curve.subs(t))) {
                    return Ok(());
                }
            }
        }
        Ok(())
    })
    .ok()
}
