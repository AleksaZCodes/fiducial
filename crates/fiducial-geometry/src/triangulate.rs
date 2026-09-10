//! Ear-clipping triangulation of a polygon with holes.
//!
//! This is the primitive every punched surface is built from. Its defining
//! property is not speed but **boundary preservation**: every boundary edge of
//! the result is an edge of an input loop, no vertex is invented, and none is
//! split. That is what lets a face with holes in it sit beside an unpunched
//! neighbour without a T-junction — the failure mode that makes a mesh
//! non-manifold while still looking watertight.
//!
//! The algorithm is the classic one: bridge each hole into the outer loop to
//! get a single (degenerate) simple polygon, then clip ears off it. No spatial
//! index, because the faces here have tens of vertices, not thousands.

extern crate alloc;

use alloc::vec::Vec;

use super::Point2;

/// Twice the signed area of triangle `p q r`.
///
/// Positive when the three points wind counter-clockwise. Used for every
/// orientation question below, so the sign convention lives in exactly one
/// place.
fn cross(p: Point2, q: Point2, r: Point2) -> f32 {
    (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)
}

/// Signed area of a closed loop: positive when wound counter-clockwise.
fn loop_area(pts: &[Point2]) -> f32 {
    let n = pts.len();
    let mut sum = 0.0f32;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
    }
    sum * 0.5
}

/// Whether the interiors of segments `a`–`b` and `c`–`d` cross.
///
/// Strictly interior: segments that merely touch at an endpoint do not count,
/// which is what a bridge landing on a boundary vertex always does.
fn interiors_cross(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let (d1, d2) = (cross(c, d, a), cross(c, d, b));
    let (d3, d4) = (cross(a, b, c), cross(a, b, d));
    ((d1 > 0.0) != (d2 > 0.0))
        && (d1 != 0.0 && d2 != 0.0)
        && ((d3 > 0.0) != (d4 > 0.0))
        && (d3 != 0.0 && d4 != 0.0)
}

/// Whether two points occupy the same location.
///
/// Exact comparison: coincident points here are literal copies made when a
/// bridge is spliced, never the result of arithmetic.
fn same(a: Point2, b: Point2) -> bool {
    a.x == b.x && a.y == b.y
}

/// Whether `p` lies on the closed segment `a`–`b`.
fn on_segment(a: Point2, b: Point2, p: Point2) -> bool {
    if cross(a, b, p) != 0.0 {
        return false;
    }
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
}

/// Whether `p` is strictly inside the closed loop `ring`, by even-odd crossing.
///
/// Safe on the bridged ring even though that ring is degenerate: a doubled
/// bridge edge contributes crossings in pairs, so parity is unaffected.
fn point_in_ring(ring: &[u32], verts: &[Point2], p: Point2) -> bool {
    let n = ring.len();
    let mut inside = false;
    for i in 0..n {
        let a = verts[ring[i] as usize];
        let b = verts[ring[(i + 1) % n] as usize];
        if (a.y > p.y) != (b.y > p.y) {
            let t = (p.y - a.y) / (b.y - a.y);
            if p.x < a.x + t * (b.x - a.x) {
                inside = !inside;
            }
        }
    }
    inside
}

/// Whether a bridge from ring position `pos` to `target` lies inside the
/// polygon.
///
/// Exact rather than heuristic. The classic ear-clipping bridge search picks a
/// target by ray-casting and a tangent heuristic, which can choose a vertex the
/// hole cannot actually see — the bridge then passes through another hole, the
/// merged ring stops being simple, and ear clipping runs out of ears and
/// silently returns a surface with a chunk missing. Faces here have tens of
/// vertices, so paying O(n) per candidate to be certain costs nothing that
/// matters.
///
/// A bridge is valid when it crosses no edge, passes through no other vertex,
/// and its midpoint is inside the polygon. With no crossings the segment is
/// wholly inside or wholly outside, so the midpoint settles which.
///
/// `pending` is every hole not yet merged, and checking against those is not
/// optional. A bridge validated only against the ring can run straight through
/// a hole that has not been merged yet — a corner fastener hole sits exactly on
/// the diagonal a rectangular annulus wants to bridge along — and the ring only
/// becomes self-intersecting later, when that hole arrives.
fn bridge_is_clear(
    ring: &[u32],
    verts: &[Point2],
    pos: usize,
    target: Point2,
    pending: &[Vec<u32>],
) -> bool {
    let from = verts[ring[pos] as usize];
    if same(from, target) {
        return false;
    }

    // Refuse a target that is already a bridge endpoint. Bridges are zero-width
    // slits, so each one pinches the polygon at both ends; let three meet at one
    // vertex and a triangle spanning the pinch can look valid edge-by-edge while
    // being topologically wrong. Ear clipping then accepts it, the ring stops
    // being simple, and the remaining surface is dropped. Keeping every pinch to
    // one slit is what makes the clip below trustworthy.
    if ring
        .iter()
        .filter(|&&v| same(verts[v as usize], from))
        .count()
        > 1
    {
        return false;
    }

    let clear_of = |loop_: &[u32]| -> bool {
        let n = loop_.len();
        for i in 0..n {
            let (a, b) = (verts[loop_[i] as usize], verts[loop_[(i + 1) % n] as usize]);
            if interiors_cross(from, target, a, b) {
                return false;
            }
            // A vertex sitting on the bridge is a degenerate pinch: allowed
            // only at the bridge's own endpoints.
            let is_endpoint = same(a, from) || same(a, target);
            if !is_endpoint && on_segment(from, target, a) {
                return false;
            }
        }
        true
    };

    if !clear_of(ring) {
        return false;
    }
    for hole in pending {
        if !clear_of(hole) {
            return false;
        }
    }

    point_in_ring(
        ring,
        verts,
        Point2::new((from.x + target.x) * 0.5, (from.y + target.y) * 0.5),
    )
}

/// The shortest clear bridge between `ring` and `hole`, as
/// `(ring position, hole position)`.
///
/// Every pairing is considered rather than only the hole's leftmost vertex.
/// With an exact visibility test some entry points have no clear bridge at all
/// while others do, so fixing the entry point in advance can fail to bridge a
/// hole that is perfectly reachable — and a hole that fails to bridge gets
/// filled in rather than punched.
fn find_bridge(
    ring: &[u32],
    verts: &[Point2],
    hole: &[u32],
    pending: &[Vec<u32>],
) -> Option<(usize, usize)> {
    let mut best: Option<((usize, usize), f32)> = None;
    for (hi, &hv) in hole.iter().enumerate() {
        let hp = verts[hv as usize];
        for pos in 0..ring.len() {
            let v = verts[ring[pos] as usize];
            let d = (v.x - hp.x) * (v.x - hp.x) + (v.y - hp.y) * (v.y - hp.y);
            if best.is_some_and(|(_, bd)| d >= bd) {
                continue;
            }
            if bridge_is_clear(ring, verts, pos, hp, pending) {
                best = Some(((pos, hi), d));
            }
        }
    }
    best.map(|(pair, _)| pair)
}

/// Splice `hole` into `ring` across a bridge, returning the merged loop.
///
/// Both bridge endpoints necessarily appear twice — that doubled edge *is* the
/// bridge, and it is what makes the merged polygon degenerate-but-simple. The
/// second copy of each gets a **fresh index** holding the same coordinates.
///
/// Reusing one index for both copies is the tempting shortcut and it is wrong:
/// several bridges can meet at one vertex, and then a single index names
/// several distinct positions in the ring. Every downstream test that asks "is
/// this vertex one of my corners" becomes ambiguous, exempts too much, and lets
/// an invalid clip through. Distinct indices keep the topology unambiguous;
/// coincident *coordinates* are then handled geometrically, where they belong.
fn splice(
    ring: &[u32],
    hole: &[u32],
    verts: &mut Vec<Point2>,
    origin: &mut Vec<u32>,
    bridge: usize,
    entry: usize,
) -> Vec<u32> {
    let hole_dup = verts.len() as u32;
    verts.push(verts[hole[entry] as usize]);
    origin.push(origin[hole[entry] as usize]);
    let ring_dup = verts.len() as u32;
    verts.push(verts[ring[bridge] as usize]);
    origin.push(origin[ring[bridge] as usize]);

    let mut out = Vec::with_capacity(ring.len() + hole.len() + 2);
    out.extend_from_slice(&ring[..=bridge]);
    for k in 0..hole.len() {
        out.push(hole[(entry + k) % hole.len()]);
    }
    out.push(hole_dup);
    out.push(ring_dup);
    out.extend_from_slice(&ring[bridge + 1..]);
    out
}

/// Whether `p` is strictly inside triangle `a b c`, which must wind CCW.
///
/// Strict, so a point on the boundary does not count. Bridging duplicates
/// vertices, so coincident points are routine here and an inclusive test would
/// let a vertex's own twin block every ear touching it.
fn strictly_inside(a: Point2, b: Point2, c: Point2, p: Point2) -> bool {
    cross(a, b, p) > 0.0 && cross(b, c, p) > 0.0 && cross(c, a, p) > 0.0
}

/// Whether the segment between `ring[i]`'s neighbours is a valid diagonal.
///
/// This is what makes ear clipping correct on a **bridged** polygon. Bridging a
/// hole leaves doubled, collinear edges, so the usual "convex corner with no
/// vertex inside" test is not enough: it will happily clip a triangle whose
/// diagonal cuts straight across a bridge. The polygon then stops being simple,
/// no further ears are found, and the surface comes out with a chunk missing.
///
/// A diagonal is valid when it crosses no edge, passes through no other vertex,
/// and lies inside the polygon. With no crossings it is wholly inside or wholly
/// outside, so its midpoint settles which.
fn diagonal_is_valid(ring: &[u32], verts: &[Point2], i: usize) -> bool {
    let n = ring.len();
    let a = verts[ring[(i + n - 1) % n] as usize];
    let c = verts[ring[(i + 1) % n] as usize];
    if same(a, c) {
        return false;
    }

    for k in 0..n {
        let (u, v) = (verts[ring[k] as usize], verts[ring[(k + 1) % n] as usize]);
        if interiors_cross(a, c, u, v) {
            return false;
        }
        // A vertex on the diagonal pinches the polygon; only points coincident
        // with the diagonal's own endpoints may sit there.
        if !same(u, a) && !same(u, c) && on_segment(a, c, u) {
            return false;
        }
    }

    point_in_ring(
        ring,
        verts,
        Point2::new((a.x + c.x) * 0.5, (a.y + c.y) * 0.5),
    )
}

/// Clip ears off a bridged polygon given as ring positions.
///
/// A corner is an ear when it turns left, no other vertex lies strictly inside
/// it, and its diagonal is valid. Reflex and collinear corners are rejected by
/// the `> 0` test, so no zero-area triangle is emitted.
fn ear_clip(ring: &[u32], verts: &[Point2]) -> Vec<[u32; 3]> {
    let mut r: Vec<u32> = ring.to_vec();
    let mut out: Vec<[u32; 3]> = Vec::with_capacity(r.len());

    while r.len() > 3 {
        let n = r.len();
        let mut clipped = None;
        'candidate: for i in 0..n {
            let (ia, ib, ic) = (r[(i + n - 1) % n], r[i], r[(i + 1) % n]);
            let (a, b, c) = (verts[ia as usize], verts[ib as usize], verts[ic as usize]);
            if cross(a, b, c) <= 0.0 {
                continue;
            }
            for k in 0..n {
                // Geometric, not by index: a bridge twin is a distinct index at
                // the same location, and it must not block an ear it is
                // effectively a corner of.
                let p = verts[r[k] as usize];
                if same(p, a) || same(p, b) || same(p, c) {
                    continue;
                }
                if strictly_inside(a, b, c, p) {
                    continue 'candidate;
                }
            }
            if !diagonal_is_valid(&r, verts, i) {
                continue;
            }
            clipped = Some((i, [ia, ib, ic]));
            break;
        }
        match clipped {
            Some((i, tri)) => {
                out.push(tri);
                r.remove(i);
            }
            // No ear on a polygon that should have one means the ring is not
            // simple. Returning what we have leaves a visible gap, which the
            // caller's area assertions report — better than spinning forever.
            // No ear on a polygon that should have one means the ring is not
            // simple. Returning what we have leaves a visible gap, which the
            // caller's area assertions report — better than spinning forever.
            // No ear on a polygon that should have one means the ring is not
            // simple. Returning what we have leaves a visible gap, which the
            // caller's area assertions report — better than spinning forever.
            None => return out,
        }
    }
    if r.len() == 3 {
        out.push([r[0], r[1], r[2]]);
    }
    out
}

/// Triangulate `outer` with `holes` punched through it.
///
/// Returns index triples into the concatenation of `outer` and each hole, in
/// the order given, so the caller maps the very points it passed in — the
/// working duplicates bridging introduces are resolved back to their originals
/// before returning, and never leak into the result.
///
/// Either winding is accepted: the outer loop is normalised counter-clockwise
/// and holes clockwise. A caller who got the winding backwards would otherwise
/// receive a triangulation that is silently inside out, which is not the kind
/// of mistake that shows up until something is printed.
pub fn triangulate(outer: &[Point2], holes: &[&[Point2]]) -> Vec<[u32; 3]> {
    if outer.len() < 3 {
        return Vec::new();
    }

    let mut verts: Vec<Point2> = Vec::with_capacity(outer.len() + 8);
    verts.extend_from_slice(outer);
    // Maps each working vertex back to the input index it stands for. Bridging
    // appends coincident duplicates to keep the ring's topology unambiguous;
    // this is what keeps them an implementation detail.
    let mut origin: Vec<u32> = (0..outer.len() as u32).collect();

    let mut ring: Vec<u32> = (0..outer.len() as u32).collect();
    if loop_area(outer) < 0.0 {
        ring.reverse();
    }

    // Each hole becomes a clockwise ring over its own slice of `verts`.
    let mut hole_rings: Vec<Vec<u32>> = Vec::with_capacity(holes.len());
    for h in holes {
        if h.len() < 3 {
            continue;
        }
        let base = verts.len() as u32;
        verts.extend_from_slice(h);
        origin.extend(base..base + h.len() as u32);
        let mut r: Vec<u32> = (base..base + h.len() as u32).collect();
        if loop_area(h) > 0.0 {
            r.reverse();
        }
        hole_rings.push(r);
    }

    // Bridge each hole into the ring. Order is not fixed in advance: a hole
    // whose only clear bridge runs through where another hole will land must be
    // merged first, so at each step take whichever hole can still be reached.
    // The exact visibility test makes that decision cheap and certain.
    while !hole_rings.is_empty() {
        let mut merged = false;
        for k in 0..hole_rings.len() {
            // Every other pending hole is an obstacle for this bridge.
            let others: Vec<Vec<u32>> = hole_rings
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != k)
                .map(|(_, h)| h.clone())
                .collect();
            if let Some((bridge, entry)) = find_bridge(&ring, &verts, &hole_rings[k], &others) {
                ring = splice(
                    &ring,
                    &hole_rings[k],
                    &mut verts,
                    &mut origin,
                    bridge,
                    entry,
                );
                hole_rings.remove(k);
                merged = true;
                break;
            }
        }
        // Nothing left is reachable: the remaining holes lie outside the outer
        // loop or touch it. Dropping them fills those holes in, which is a
        // visible wrong answer rather than a broken surface.
        if !merged {
            break;
        }
    }

    ear_clip(&ring, &verts)
        .into_iter()
        .map(|t| {
            [
                origin[t[0] as usize],
                origin[t[1] as usize],
                origin[t[2] as usize],
            ]
        })
        .collect()
}

/// A closed regular polygon approximating a circle, wound counter-clockwise.
///
/// `segments` is clamped to at least 3. The polygon is **inscribed**, so it is
/// slightly smaller than the true circle — for a hole that means a slightly
/// tight fit, which is the safe direction to be wrong when the alternative is a
/// screw that no longer grips.
pub fn circle(center: Point2, radius: f32, segments: usize) -> Vec<Point2> {
    let n = segments.max(3);
    let mut pts = Vec::with_capacity(n);
    for i in 0..n {
        let t = core::f32::consts::TAU * (i as f32) / (n as f32);
        pts.push(Point2::new(
            center.x + radius * libm::cosf(t),
            center.y + radius * libm::sinf(t),
        ));
    }
    pts
}

/// Radius a polygon needs so that its **inscribed** circle is `radius`.
///
/// A regular polygon through the circle's points sits inside it, so a hole
/// built that way is undersized by `1 / cos(pi / n)`. Scaling the construction
/// radius up by that factor makes the flat-to-flat distance equal the diameter
/// asked for, which is what a screw actually has to pass through.
pub fn circumradius_for_width(radius: f32, segments: usize) -> f32 {
    let n = segments.max(3) as f32;
    radius / libm::cosf(core::f32::consts::PI / n)
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::{collections::HashMap, vec, vec::Vec};

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Point2> {
        vec![
            Point2::new(x0, y0),
            Point2::new(x1, y0),
            Point2::new(x1, y1),
            Point2::new(x0, y1),
        ]
    }

    /// Rebuild the vertex list `triangulate` indexes into.
    fn flatten(outer: &[Point2], holes: &[&[Point2]]) -> Vec<Point2> {
        let mut v = outer.to_vec();
        for h in holes {
            v.extend_from_slice(h);
        }
        v
    }

    fn total_area(tris: &[[u32; 3]], verts: &[Point2]) -> f32 {
        tris.iter()
            .map(|t| {
                let (a, b, c) = (
                    verts[t[0] as usize],
                    verts[t[1] as usize],
                    verts[t[2] as usize],
                );
                cross(a, b, c) * 0.5
            })
            .sum()
    }

    /// A point quantised so coincident vertices compare equal.
    type PointKey = (i64, i64);
    /// An undirected edge between two quantised points.
    type EdgeKey = (PointKey, PointKey);

    fn key(p: Point2) -> PointKey {
        ((p.x * 4096.0).round() as i64, (p.y * 4096.0).round() as i64)
    }

    fn undirected(a: Point2, b: Point2) -> EdgeKey {
        let (ka, kb) = (key(a), key(b));
        if ka <= kb {
            (ka, kb)
        } else {
            (kb, ka)
        }
    }

    /// The edges used exactly once by the triangulation — its boundary.
    ///
    /// Interior edges are shared by two triangles, and a hole bridge is
    /// traversed once in each direction, so both are used twice. What is left
    /// must be precisely the input loops.
    fn boundary_edges(tris: &[[u32; 3]], verts: &[Point2]) -> Vec<((i64, i64), (i64, i64))> {
        let mut counts: HashMap<EdgeKey, usize> = HashMap::new();
        for t in tris {
            for k in 0..3 {
                let a = verts[t[k] as usize];
                let b = verts[t[(k + 1) % 3] as usize];
                *counts.entry(undirected(a, b)).or_insert(0) += 1;
            }
        }
        let mut out: Vec<_> = counts
            .into_iter()
            .filter(|(_, c)| *c == 1)
            .map(|(e, _)| e)
            .collect();
        out.sort();
        out
    }

    fn loop_edges(loops: &[&[Point2]]) -> Vec<EdgeKey> {
        let mut out = Vec::new();
        for l in loops {
            for i in 0..l.len() {
                out.push(undirected(l[i], l[(i + 1) % l.len()]));
            }
        }
        out.sort();
        out
    }

    /// Assert the triangulation covers the polygon exactly and invents no edge.
    fn assert_sound(outer: &[Point2], holes: &[&[Point2]]) -> Vec<[u32; 3]> {
        let verts = flatten(outer, holes);
        let tris = triangulate(outer, holes);
        assert!(!tris.is_empty(), "triangulation produced nothing");

        // Every triangle wound counter-clockwise, none degenerate.
        for t in &tris {
            let (a, b, c) = (
                verts[t[0] as usize],
                verts[t[1] as usize],
                verts[t[2] as usize],
            );
            let area2 = cross(a, b, c);
            assert!(
                area2 > 0.0,
                "triangle {t:?} has signed area {}",
                area2 * 0.5
            );
        }

        // Area is conserved: the covered area is the outer loop less its holes.
        let expected =
            loop_area(outer).abs() - holes.iter().map(|h| loop_area(h).abs()).sum::<f32>();
        let actual = total_area(&tris, &verts);
        let tol = expected.abs() * 1e-3 + 1e-3;
        assert!(
            (actual - expected).abs() < tol,
            "area {actual} != expected {expected}"
        );

        // The boundary is exactly the input loops — the property that keeps a
        // punched face flush with its unpunched neighbours.
        let mut all: Vec<&[Point2]> = vec![outer];
        all.extend_from_slice(holes);
        assert_eq!(
            boundary_edges(&tris, &verts),
            loop_edges(&all),
            "triangulation boundary differs from the input loops"
        );
        tris
    }

    #[test]
    fn a_rectangle_becomes_two_triangles() {
        let r = rect(0.0, 0.0, 10.0, 4.0);
        let tris = assert_sound(&r, &[]);
        assert_eq!(tris.len(), 2);
    }

    #[test]
    fn winding_is_normalised() {
        // A caller who passes the loops backwards must still get a usable
        // triangulation, not one that is silently inside out.
        let mut ccw = rect(0.0, 0.0, 10.0, 4.0);
        let cw: Vec<Point2> = ccw.iter().rev().copied().collect();
        assert!(loop_area(&ccw) > 0.0 && loop_area(&cw) < 0.0);
        assert_sound(&cw, &[]);
        ccw.reverse();
        assert_sound(&ccw, &[]);
    }

    #[test]
    fn a_rectangular_ring() {
        let outer = rect(0.0, 0.0, 20.0, 12.0);
        let hole = rect(4.0, 3.0, 16.0, 9.0);
        assert_sound(&outer, &[&hole]);
    }

    #[test]
    fn a_round_hole_in_a_rectangle() {
        let outer = rect(0.0, 0.0, 20.0, 12.0);
        let hole = circle(Point2::new(10.0, 6.0), 3.0, 24);
        assert_sound(&outer, &[&hole]);
    }

    #[test]
    fn four_round_holes_in_a_rectangular_ring() {
        // The rim of a fastened case: a ring with a screw hole at each corner,
        // each sitting exactly on the mitre diagonal the old four-trapezoid
        // ring was split along. One polygon, so the mitre is not a seam.
        let outer = rect(0.0, 0.0, 110.0, 70.0);
        let inner = rect(7.0, 7.0, 103.0, 63.0);
        let holes: Vec<Vec<Point2>> = [(2.9, 2.9), (107.1, 2.9), (107.1, 67.1), (2.9, 67.1)]
            .iter()
            .map(|(x, y)| circle(Point2::new(*x, *y), 1.7, 20))
            .collect();
        let mut refs: Vec<&[Point2]> = vec![&inner];
        refs.extend(holes.iter().map(|h| h.as_slice()));
        assert_sound(&outer, &refs);
    }

    #[test]
    fn many_holes_in_one_face() {
        let outer = rect(0.0, 0.0, 60.0, 40.0);
        let holes: Vec<Vec<Point2>> = (0..8)
            .map(|i| {
                let x = 6.0 + (i % 4) as f32 * 14.0;
                let y = 10.0 + (i / 4) as f32 * 18.0;
                if i % 2 == 0 {
                    circle(Point2::new(x, y), 3.0, 16)
                } else {
                    rect(x - 3.0, y - 3.0, x + 3.0, y + 3.0)
                }
            })
            .collect();
        let refs: Vec<&[Point2]> = holes.iter().map(|h| h.as_slice()).collect();
        assert_sound(&outer, &refs);
    }

    #[test]
    fn holes_sharing_an_axis_do_not_confuse_the_bridge() {
        // Two holes at the same x force the bridge search to break a tie, and
        // two at the same y put a hole's entry point level with another's.
        let outer = rect(0.0, 0.0, 40.0, 40.0);
        let a = circle(Point2::new(10.0, 10.0), 3.0, 12);
        let b = circle(Point2::new(10.0, 30.0), 3.0, 12);
        let c = circle(Point2::new(30.0, 10.0), 3.0, 12);
        assert_sound(&outer, &[&a, &b, &c]);
    }

    #[test]
    fn a_degenerate_loop_yields_nothing_rather_than_garbage() {
        assert!(triangulate(&[], &[]).is_empty());
        assert!(triangulate(&[Point2::new(0.0, 0.0)], &[]).is_empty());
        let two = [Point2::new(0.0, 0.0), Point2::new(1.0, 1.0)];
        assert!(triangulate(&two, &[]).is_empty());
    }

    #[test]
    fn a_hole_with_too_few_points_is_ignored() {
        let outer = rect(0.0, 0.0, 10.0, 10.0);
        let stub = [Point2::new(4.0, 4.0), Point2::new(5.0, 5.0)];
        // Filled in rather than punched: two points do not bound an area.
        let tris = triangulate(&outer, &[&stub]);
        let verts = flatten(&outer, &[&stub]);
        assert!((total_area(&tris, &verts) - 100.0).abs() < 1e-2);
    }

    #[test]
    fn fuzz_random_hole_layouts_stay_sound() {
        // Deterministic xorshift — a fixed sequence so a failure is reproducible.
        let mut state = 0x2545_F491u32;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 8) as f32 / 16_777_216.0
        };

        for case in 0..3000 {
            let (w, h) = (40.0 + next() * 60.0, 30.0 + next() * 40.0);
            let outer = rect(0.0, 0.0, w, h);
            let count = 1 + (next() * 5.0) as usize;

            // Lay holes out on a coarse grid so they cannot overlap; an
            // overlapping hole is not a valid input and is rejected upstream.
            let cols = 3usize;
            let mut holes: Vec<Vec<Point2>> = Vec::new();
            for k in 0..count {
                let (cx, cy) = (
                    w * (0.5 + (k % cols) as f32) / cols as f32,
                    h * (0.5 + (k / cols) as f32) / 3.0,
                );
                let r = 1.5 + next() * 2.5;
                holes.push(if next() < 0.5 {
                    circle(Point2::new(cx, cy), r, 8 + (next() * 16.0) as usize)
                } else {
                    rect(cx - r, cy - r, cx + r, cy + r)
                });
            }
            let refs: Vec<&[Point2]> = holes.iter().map(|x| x.as_slice()).collect();

            let verts = flatten(&outer, &refs);
            let tris = triangulate(&outer, &refs);
            let expected = w * h - refs.iter().map(|x| loop_area(x).abs()).sum::<f32>();
            let actual = total_area(&tris, &verts);
            assert!(
                (actual - expected).abs() < expected * 1e-3 + 1e-2,
                "case {case}: area {actual} != {expected} ({} holes)",
                refs.len()
            );
            for t in &tris {
                let area2 = cross(
                    verts[t[0] as usize],
                    verts[t[1] as usize],
                    verts[t[2] as usize],
                );
                assert!(area2 > 0.0, "case {case}: triangle {t:?} is not CCW");
            }
            let mut all: Vec<&[Point2]> = vec![&outer];
            all.extend_from_slice(&refs);
            assert_eq!(
                boundary_edges(&tris, &verts),
                loop_edges(&all),
                "case {case}: boundary differs from the input loops"
            );
        }
    }

    #[test]
    fn adversarial_annulus_layouts_stay_sound() {
        // The rim of a fastened case, swept across plausible geometry: every
        // corner hole sits on the mitre diagonal, which is the arrangement that
        // broke every earlier version of the bridge search.
        for &(w, h) in &[
            (110.0f32, 70.0f32),
            (40.0, 40.0),
            (200.0, 30.0),
            (25.0, 90.0),
        ] {
            for &border in &[5.0f32, 7.0, 9.5] {
                for &segs in &[8usize, 12, 20, 32] {
                    let outer = rect(0.0, 0.0, w, h);
                    let inner = rect(border, border, w - border, h - border);
                    let r = border * 0.24;
                    let c = border * 0.5;
                    let centres = [(c, c), (w - c, c), (w - c, h - c), (c, h - c)];
                    let holes: Vec<Vec<Point2>> = centres
                        .iter()
                        .map(|(x, y)| circle(Point2::new(*x, *y), r, segs))
                        .collect();
                    let mut refs: Vec<&[Point2]> = std::vec![&inner];
                    refs.extend(holes.iter().map(|x| x.as_slice()));
                    assert_sound(&outer, &refs);
                }
            }
        }
    }

    #[test]
    fn a_wall_with_openings_at_many_offsets_stays_sound() {
        // A case wall, swept over opening positions and counts.
        for n in 1usize..=4 {
            for shift in 0..6 {
                let outer = rect(0.0, 0.0, 120.0, 16.0);
                let holes: Vec<Vec<Point2>> = (0..n)
                    .map(|k| {
                        let x = 10.0 + k as f32 * 28.0 + shift as f32 * 1.7;
                        rect(
                            x,
                            3.0 + (k % 2) as f32 * 1.5,
                            x + 9.0,
                            7.0 + (k % 2) as f32 * 1.5,
                        )
                    })
                    .collect();
                let refs: Vec<&[Point2]> = holes.iter().map(|x| x.as_slice()).collect();
                assert_sound(&outer, &refs);
            }
        }
    }

    #[test]
    fn holes_touching_the_outer_boundary_are_still_handled() {
        // A hole flush against the outer edge: the bridge has nowhere to go but
        // along the boundary, and the triangulation must either punch it or
        // leave it, never emit a broken surface.
        let outer = rect(0.0, 0.0, 20.0, 10.0);
        let flush = rect(0.0, 3.0, 4.0, 7.0);
        let verts = flatten(&outer, &[&flush]);
        let tris = triangulate(&outer, &[&flush]);
        for t in &tris {
            let area2 = cross(
                verts[t[0] as usize],
                verts[t[1] as usize],
                verts[t[2] as usize],
            );
            assert!(area2 > 0.0, "triangle {t:?} is not CCW");
        }
        let a = total_area(&tris, &verts);
        assert!(
            a <= 200.0 + 1e-2,
            "cannot cover more than the outer loop: {a}"
        );
        assert!(a > 0.0);
    }

    #[test]
    fn circle_is_closed_regular_and_counter_clockwise() {
        let c = circle(Point2::new(1.0, 2.0), 5.0, 16);
        assert_eq!(c.len(), 16);
        assert!(loop_area(&c) > 0.0, "circle must wind counter-clockwise");
        for p in &c {
            let d = libm::sqrtf((p.x - 1.0) * (p.x - 1.0) + (p.y - 2.0) * (p.y - 2.0));
            assert!((d - 5.0).abs() < 1e-4, "radius {d}");
        }
        // Inscribed, so it undershoots the true circle's area.
        let ideal = core::f32::consts::PI * 25.0;
        assert!(loop_area(&c) < ideal);
    }

    #[test]
    fn circle_segments_are_clamped_to_a_real_polygon() {
        for n in [0, 1, 2, 3] {
            assert_eq!(circle(Point2::new(0.0, 0.0), 1.0, n).len(), 3.max(n));
        }
    }

    #[test]
    fn circumradius_makes_the_flats_reach_the_asked_for_width() {
        // A screw passes through the flats, not the corners, so an inscribed
        // polygon is undersized. Correcting the construction radius makes the
        // narrowest width equal the diameter requested.
        for n in [8, 12, 16, 24, 32] {
            let want_r = 1.7;
            let r = circumradius_for_width(want_r, n);
            assert!(r >= want_r, "correction must not shrink the hole");
            let pts = circle(Point2::new(0.0, 0.0), r, n);
            // Narrowest half-width: the apothem, midpoint of any edge.
            let mid = Point2::new((pts[0].x + pts[1].x) * 0.5, (pts[0].y + pts[1].y) * 0.5);
            let apothem = libm::sqrtf(mid.x * mid.x + mid.y * mid.y);
            assert!(
                (apothem - want_r).abs() < 1e-4,
                "n={n}: apothem {apothem} != {want_r}"
            );
        }
    }
}
