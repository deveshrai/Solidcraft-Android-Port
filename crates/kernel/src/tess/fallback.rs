//! The last resort for a face the planner cannot plan however finely its edges are divided: a
//! leftover of the file's own modelling, such as a cusp where two edges leave a point at a zero
//! angle, a sliver whose two sides sag across each other, or a small cap round a pole. Its
//! boundary is filled flat, so the rest of the body keeps its plan rather than the whole body
//! falling back to the reference mesh.
//!
//! The boundary nodes (the shared edge nodes, so the face still meets its neighbours exactly)
//! are laid onto the plane that best fits them and triangulated there (see `cdt::triangulate`):
//! every boundary side is kept as a constraint unless it crosses one already kept (the defect
//! itself), and a triangle is in the face when it lies inside an odd number of loops. The fill is
//! kept only where it stays within the chord tolerance of the surface, so a real curved face is
//! left to the body's reference mesh. The triangles turn the way the surface does, and each node
//! keeps its parameters on the surface for its normal.

use solidcraft_geom::tess::{EvalSurface, FacePlan};
use solidcraft_geom::{Plane, Vec3};

use super::{cdt, uv};

/// Most boundary nodes a face is filled at.
const MAX_NODES: usize = 50_000;
/// Nodes the surface's normal is sampled at to find which way it faces.
const NORMAL_SAMPLES: usize = 16;

/// The face `fi` of surface `s`, bounded by the loops `loops` (in space), filled flat; `None`
/// when its nodes do not lie on the surface within `accept`, span no plane, enclose nothing, or
/// the flat fill strays from the surface by more than the chord tolerance `tol` (a real curved
/// face, which the body's reference mesh draws better).
pub(crate) fn fill(fi: usize, s: &EvalSurface, loops: &[Vec<Vec3>], accept: f64, tol: f64) -> Option<FacePlan> {
    let fixed: Vec<Vec3> = loops.iter().flatten().copied().collect();
    if fixed.len() < 3 || fixed.len() > MAX_NODES {
        return None;
    }
    // Each node's parameters, for its normal (a projection from the previous node's).
    let dom = uv::domain(s);
    let mut params: Vec<[f64; 2]> = Vec::with_capacity(fixed.len());
    let mut ranges = Vec::with_capacity(loops.len());
    for l in loops {
        let start = params.len();
        let mut prev = None;
        for p in l {
            let q = uv::project(s, &dom, *p, prev, accept)?;
            params.push(q);
            prev = Some(q);
        }
        ranges.push(start..params.len());
    }
    // The plane: the loops' area vector (holes count against the outer loop) through their
    // centroid.
    let normal = loops.iter().fold(Vec3::ZERO, |n, l| n + crate::plane_fit::loop_area(l));
    let centre = fixed.iter().fold(Vec3::ZERO, |c, p| c + *p) * (1.0 / fixed.len() as f64);
    let plane = Plane::from_normal(centre, normal)?;
    let flat: Vec<[f64; 2]> = fixed.iter().map(|p| plane.to_local(*p)).map(|q| [q.x, q.y]).collect();
    let fill = cdt::Fill { seeds: &[], refine: None, shape: false, lenient: true };
    let (_, mut triangles) = cdt::triangulate(&flat, &ranges, fill)?;
    if triangles.is_empty() {
        return None;
    }
    // Flat only where flat is close enough: each triangle's middle within the chord tolerance of
    // the surface (found from its first corner's parameters).
    let off = |t: &[u32; 3]| -> Option<f64> {
        let [a, b, c] = t.map(|i| fixed.get(i as usize).copied());
        let mid = (a? + b? + c?) * (1.0 / 3.0);
        let q = uv::project(s, &dom, mid, params.get(t[0] as usize).copied(), f64::INFINITY)?;
        Some(s.derivs(q[0], q[1])?.0.dist(mid))
    };
    if !triangles.iter().all(|t| off(t).is_some_and(|d| d <= tol)) {
        return None;
    }
    // Counter-clockwise about the plane's normal; turned over where the surface faces the other way.
    let step = (params.len() / NORMAL_SAMPLES).max(1);
    let along: f64 = params.iter().step_by(step).filter_map(|q| s.derivs(q[0], q[1])).map(|(_, su, sv)| su.cross(sv).dot(plane.normal())).sum();
    if along < 0.0 {
        for t in &mut triangles {
            t.swap(1, 2);
        }
    }
    Some(FacePlan { face: u32::try_from(fi).unwrap_or(u32::MAX), surface: None, uv: params, fixed, normals: Vec::new(), flip: false, triangles })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plane face whose right side folds back over itself at a zero angle (a cusp): no side
    /// crossing another may be kept, yet the face is filled and turns the plane's way.
    #[test]
    fn a_cusp_is_filled_the_surfaces_way() {
        let s = EvalSurface::Plane { origin: Vec3::ZERO, du: Vec3::X, dv: Vec3::Y };
        let corners = [[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [3.0, 4.2], [3.8, 3.6], [0.0, 4.0]];
        let l: Vec<Vec3> = corners.iter().map(|q| Vec3::new(q[0], q[1], 0.0)).collect();
        let Some(f) = fill(0, &s, &[l], 1e-9, 1e-9) else { panic!("filled") };
        assert!(!f.triangles.is_empty());
        let area: f64 = f
            .triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| f.fixed[i as usize]);
                (b - a).cross(c - a).z * 0.5
            })
            .sum();
        assert!(area > 14.0 && area < 16.5, "area {area}");
        assert_eq!(f.uv.len(), f.fixed.len());
    }

    #[test]
    fn a_boundary_enclosing_nothing_is_not_filled() {
        let s = EvalSurface::Plane { origin: Vec3::ZERO, du: Vec3::X, dv: Vec3::Y };
        let l = vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)];
        assert!(fill(0, &s, &[l], 1e-3, 1e-3).is_none());
    }
}
