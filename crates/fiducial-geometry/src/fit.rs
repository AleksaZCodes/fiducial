//! Will this body fit there? — exact fit tests on arbitrary simple polygons.
//!
//! The enclosure generator in `fiducial-mesh` sizes a case from a bounding
//! box, which is only right when the outline *is* a box. A product whose
//! outline is a logo is not, and the questions a physical product actually asks
//! are all of one shape:
//!
//! - does a `w × h` body fit inside this outline, at least `clearance` from
//!   every wall? ([`rect_fits`])
//! - where is the best place for it, given what is already placed?
//!   ([`place_rect`])
//! - where are the corners a screw can sit in? ([`offset_vertices`])
//!
//! Every function here works on **non-convex** polygons. The fit test is exact
//! rather than sampled: a rectangle is inside a simple polygon, at least `c`
//! from its boundary, iff one corner is inside and no rectangle edge comes
//! within `c` of any polygon edge. Placement *search* is sampled on a declared
//! grid, and says so — the result is always a position that passes the exact
//! test, never an approximation of one.
//!
//! `f64` throughout, unlike the rest of this crate: these answers are compared
//! against manufacturing tolerances of a tenth of a millimetre on parts tens
//! of centimetres long, which is where `f32` stops being boring.
//!
//! Polygons are counter-clockwise, y up, in millimetres.

extern crate alloc;

use alloc::vec::Vec;

/// A point, `(x, y)`.
pub type P = (f64, f64);

/// An axis-aligned rectangle by centre and full size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Centre.
    pub centre: P,
    /// Full width (x) and height (y).
    pub size: P,
}

impl Rect {
    /// The four corners, counter-clockwise from bottom-left.
    pub fn corners(&self) -> [P; 4] {
        let (cx, cy) = self.centre;
        let (hw, hh) = (self.size.0 / 2.0, self.size.1 / 2.0);
        [
            (cx - hw, cy - hh),
            (cx + hw, cy - hh),
            (cx + hw, cy + hh),
            (cx - hw, cy + hh),
        ]
    }

    /// Do two rectangles come within `gap` of each other?
    pub fn overlaps(&self, other: &Rect, gap: f64) -> bool {
        (self.centre.0 - other.centre.0).abs() < (self.size.0 + other.size.0) / 2.0 + gap
            && (self.centre.1 - other.centre.1).abs() < (self.size.1 + other.size.1) / 2.0 + gap
    }
}

/// Twice the signed area; positive when counter-clockwise.
pub fn signed_area2(poly: &[P]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum()
}

/// Area-weighted centroid.
pub fn centroid(poly: &[P]) -> P {
    let a2 = signed_area2(poly);
    let n = poly.len();
    let (mut cx, mut cy) = (0.0, 0.0);
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let w = a.0 * b.1 - b.0 * a.1;
        cx += (a.0 + b.0) * w;
        cy += (a.1 + b.1) * w;
    }
    (cx / (3.0 * a2), cy / (3.0 * a2))
}

/// Axis-aligned bounds: `(min, max)`.
pub fn bounds(poly: &[P]) -> (P, P) {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for &(x, y) in poly {
        lo = (lo.0.min(x), lo.1.min(y));
        hi = (hi.0.max(x), hi.1.max(y));
    }
    (lo, hi)
}

/// Even-odd point-in-polygon. Points exactly on the boundary are unspecified;
/// every caller here also requires a positive distance from it.
pub fn contains(poly: &[P], p: P) -> bool {
    let n = poly.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn dist_point_seg(p: P, a: P, b: P) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    libm::hypot(p.0 - (a.0 + t * dx), p.1 - (a.1 + t * dy))
}

fn orient(a: P, b: P, c: P) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn segments_cross(a: P, b: P, c: P, d: P) -> bool {
    let (o1, o2) = (orient(a, b, c), orient(a, b, d));
    let (o3, o4) = (orient(c, d, a), orient(c, d, b));
    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}

/// Shortest distance between two segments.
pub fn dist_seg_seg(a: P, b: P, c: P, d: P) -> f64 {
    if segments_cross(a, b, c, d) {
        return 0.0;
    }
    dist_point_seg(a, c, d)
        .min(dist_point_seg(b, c, d))
        .min(dist_point_seg(c, a, b))
        .min(dist_point_seg(d, a, b))
}

/// Shortest distance from a point to the polygon's boundary.
pub fn dist_to_boundary(poly: &[P], p: P) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| dist_point_seg(p, poly[i], poly[(i + 1) % n]))
        .fold(f64::INFINITY, f64::min)
}

/// How far inside the polygon's boundary the rectangle stays: the smallest
/// distance from any rectangle edge to any polygon edge, **negative** when any
/// part of the rectangle is outside. Exact for any simple polygon.
pub fn rect_margin(poly: &[P], r: &Rect) -> f64 {
    let c = r.corners();
    if !contains(poly, c[0]) {
        return -1.0;
    }
    let n = poly.len();
    let mut m = f64::INFINITY;
    for i in 0..4 {
        let (a, b) = (c[i], c[(i + 1) % 4]);
        for j in 0..n {
            let d = dist_seg_seg(a, b, poly[j], poly[(j + 1) % n]);
            if d == 0.0 {
                return -1.0;
            }
            m = m.min(d);
        }
    }
    // One corner inside and no edge crossing leaves one case: the polygon lies
    // entirely inside the rectangle. Then every polygon vertex is inside it.
    let (lo, hi) = (c[0], c[2]);
    if poly
        .iter()
        .all(|&(x, y)| x > lo.0 && x < hi.0 && y > lo.1 && y < hi.1)
    {
        return -1.0;
    }
    m
}

/// Does the rectangle fit inside the polygon, at least `clearance` from every wall?
pub fn rect_fits(poly: &[P], r: &Rect, clearance: f64) -> bool {
    rect_margin(poly, r) >= clearance
}

/// The first placement, nearest `target`, at which a `size` rectangle fits
/// `poly` with `clearance` and stays `gap` away from every rectangle in
/// `avoid`. Candidates are centres on a `step` grid anchored at `target`, so
/// the answer is deterministic and reproducible; `None` means nothing on that
/// grid fits, which for a sensible step means nothing fits.
pub fn place_rect(
    poly: &[P],
    size: P,
    clearance: f64,
    avoid: &[Rect],
    gap: f64,
    target: P,
    step: f64,
) -> Option<Rect> {
    place_rect_candidates(poly, size, clearance, avoid, gap, target, step, 1, 0.0)
        .into_iter()
        .next()
}

/// Up to `max` placements that fit, nearest `target` first, each at least
/// `spacing` from the ones before it — the alternatives a search can fall back
/// on when the nearest spot leaves no room for what comes after.
#[allow(clippy::too_many_arguments)]
pub fn place_rect_candidates(
    poly: &[P],
    size: P,
    clearance: f64,
    avoid: &[Rect],
    gap: f64,
    target: P,
    step: f64,
    max: usize,
    spacing: f64,
) -> Vec<Rect> {
    let mut out: Vec<Rect> = Vec::new();
    let (lo, hi) = bounds(poly);
    let reach = libm::fmax(hi.0 - lo.0, hi.1 - lo.1);
    let rings = libm::ceil(reach / step) as i64 + 1;
    // Rings of increasing Chebyshev radius; inside a ring, nearest first.
    for k in 0..=rings {
        let mut ring: Vec<(f64, P)> = Vec::new();
        for i in -k..=k {
            for j in -k..=k {
                if i.abs() != k && j.abs() != k {
                    continue;
                }
                let c = (target.0 + i as f64 * step, target.1 + j as f64 * step);
                let d = libm::hypot(c.0 - target.0, c.1 - target.1);
                ring.push((d, c));
            }
        }
        ring.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap()
                .then(a.1 .1.partial_cmp(&b.1 .1).unwrap())
                .then(a.1 .0.partial_cmp(&b.1 .0).unwrap())
        });
        for (_, c) in ring {
            let r = Rect { centre: c, size };
            // Cheap first: a rectangle poking out of the polygon's bounds
            // cannot fit inside it. Most candidates for a large rectangle
            // fail here, before the exact test.
            if c.0 - size.0 / 2.0 < lo.0 + clearance
                || c.0 + size.0 / 2.0 > hi.0 - clearance
                || c.1 - size.1 / 2.0 < lo.1 + clearance
                || c.1 + size.1 / 2.0 > hi.1 - clearance
            {
                continue;
            }
            if out
                .iter()
                .any(|o| libm::hypot(o.centre.0 - c.0, o.centre.1 - c.1) < spacing)
            {
                continue;
            }
            if avoid.iter().any(|o| r.overlaps(o, gap)) {
                continue;
            }
            if rect_fits(poly, &r, clearance) {
                out.push(r);
                if out.len() >= max {
                    return out;
                }
            }
        }
    }
    out
}

/// Interior angle at each vertex in degrees; above 180 is a reflex (inward) corner.
pub fn interior_angles(poly: &[P]) -> Vec<f64> {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (p, c, q) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
            let a1 = libm::atan2(p.1 - c.1, p.0 - c.0);
            let a2 = libm::atan2(q.1 - c.1, q.0 - c.0);
            let mut d = (a1 - a2).to_degrees();
            while d < 0.0 {
                d += 360.0;
            }
            while d >= 360.0 {
                d -= 360.0;
            }
            d
        })
        .collect()
}

/// Each vertex moved inward by `d` along the corner's bisector, so it sits `d`
/// from both edges that meet there. `None` for a reflex vertex, where "`d`
/// from both edges" puts the point outside the material.
///
/// This is the position of a screw in a corner: the one place in a wall where
/// a boss is supported on two sides.
pub fn offset_vertices(poly: &[P], d: f64) -> Vec<Option<P>> {
    let n = poly.len();
    let angles = interior_angles(poly);
    (0..n)
        .map(|i| {
            if angles[i] >= 180.0 - 1e-9 {
                return None;
            }
            let (p, c, q) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
            let u = norm((p.0 - c.0, p.1 - c.1));
            let v = norm((q.0 - c.0, q.1 - c.1));
            let bis = norm((u.0 + v.0, u.1 + v.1));
            let half = angles[i].to_radians() / 2.0;
            let along = d / libm::sin(half);
            let pt = (c.0 + bis.0 * along, c.1 + bis.1 * along);
            (contains(poly, pt) && dist_to_boundary(poly, pt) >= d - 1e-6).then_some(pt)
        })
        .collect()
}

fn norm(v: P) -> P {
    let l = libm::hypot(v.0, v.1);
    (v.0 / l, v.1 / l)
}

/// Perimeter length.
pub fn perimeter(poly: &[P]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            libm::hypot(
                poly[(i + 1) % n].0 - poly[i].0,
                poly[(i + 1) % n].1 - poly[i].1,
            )
        })
        .sum()
}

/// Pick `count` of `candidates`, spread as far apart as possible: greedy
/// farthest-point sampling, starting from the candidate with the lowest y then
/// x. Deterministic, and returns indices in their original order.
pub fn spread(candidates: &[P], count: usize) -> Vec<usize> {
    if candidates.is_empty() || count == 0 {
        return Vec::new();
    }
    let mut first = 0;
    for (i, c) in candidates.iter().enumerate() {
        let f = candidates[first];
        if c.1 < f.1 - 1e-9 || ((c.1 - f.1).abs() <= 1e-9 && c.0 < f.0) {
            first = i;
        }
    }
    let mut chosen = alloc::vec![first];
    while chosen.len() < count.min(candidates.len()) {
        let mut best = (f64::NEG_INFINITY, 0);
        for (i, c) in candidates.iter().enumerate() {
            if chosen.contains(&i) {
                continue;
            }
            let d = chosen
                .iter()
                .map(|&j| libm::hypot(c.0 - candidates[j].0, c.1 - candidates[j].1))
                .fold(f64::INFINITY, f64::min);
            if d > best.0 + 1e-9 {
                best = (d, i);
            }
        }
        chosen.push(best.1);
    }
    chosen.sort_unstable();
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// An L: 0..20 × 0..10 plus 0..10 × 10..20. Non-convex at (10, 10).
    fn ell() -> Vec<P> {
        vec![
            (0.0, 0.0),
            (20.0, 0.0),
            (20.0, 10.0),
            (10.0, 10.0),
            (10.0, 20.0),
            (0.0, 20.0),
        ]
    }

    fn rect(cx: f64, cy: f64, w: f64, h: f64) -> Rect {
        Rect {
            centre: (cx, cy),
            size: (w, h),
        }
    }

    #[test]
    fn area_and_centroid_of_a_square() {
        let sq = vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];
        assert_eq!(signed_area2(&sq), 32.0);
        assert_eq!(centroid(&sq), (2.0, 2.0));
    }

    #[test]
    fn a_rect_in_one_arm_of_the_l_fits() {
        assert!(rect_fits(&ell(), &rect(15.0, 5.0, 8.0, 8.0), 0.5));
        assert!((rect_margin(&ell(), &rect(15.0, 5.0, 8.0, 8.0)) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_rect_across_the_reflex_corner_does_not() {
        // All four corners inside the L's bounding box, one outside the L itself.
        assert!(!rect_fits(&ell(), &rect(12.0, 12.0, 6.0, 6.0), 0.0));
        // All four corners inside the L, but an edge crosses the notch.
        let r = rect(9.0, 9.0, 16.0, 16.0);
        assert!(r.corners().iter().filter(|&&c| contains(&ell(), c)).count() >= 3);
        assert!(!rect_fits(&ell(), &r, 0.0));
    }

    #[test]
    fn clearance_is_enforced_exactly() {
        let r = rect(15.0, 5.0, 8.0, 8.0); // 1 mm from three walls
        assert!(rect_fits(&ell(), &r, 1.0 - 1e-9));
        assert!(!rect_fits(&ell(), &r, 1.0 + 1e-9));
    }

    #[test]
    fn a_polygon_inside_the_rect_is_not_a_fit() {
        let tiny = vec![(1.0, 1.0), (2.0, 1.0), (2.0, 2.0), (1.0, 2.0)];
        assert!(!rect_fits(&tiny, &rect(1.5, 1.5, 10.0, 10.0), 0.0));
    }

    #[test]
    fn placement_finds_the_arm_and_avoids_what_is_placed() {
        let r = place_rect(&ell(), (8.0, 8.0), 0.5, &[], 0.0, (10.0, 10.0), 0.5).unwrap();
        assert!(rect_fits(&ell(), &r, 0.5));
        let blocker = r;
        let r2 = place_rect(&ell(), (8.0, 8.0), 0.5, &[blocker], 1.0, (10.0, 10.0), 0.5).unwrap();
        assert!(!r2.overlaps(&blocker, 1.0));
        assert!(rect_fits(&ell(), &r2, 0.5));
    }

    #[test]
    fn placement_says_none_when_nothing_fits() {
        assert!(place_rect(&ell(), (15.0, 15.0), 0.0, &[], 0.0, (10.0, 10.0), 0.5).is_none());
    }

    #[test]
    fn the_reflex_corner_is_found_and_gets_no_screw() {
        let a = interior_angles(&ell());
        assert!((a[3] - 270.0).abs() < 1e-9);
        assert!((a[0] - 90.0).abs() < 1e-9);
        let v = offset_vertices(&ell(), 2.0);
        assert!(v[3].is_none());
        let p = v[0].unwrap();
        assert!((p.0 - 2.0).abs() < 1e-9 && (p.1 - 2.0).abs() < 1e-9);
    }

    #[test]
    fn spread_picks_far_apart_points() {
        let pts = vec![
            (0.0, 0.0),
            (1.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
        ];
        assert_eq!(spread(&pts, 2), vec![0, 3]);
        assert_eq!(spread(&pts, 4), vec![0, 2, 3, 4]);
    }
}
