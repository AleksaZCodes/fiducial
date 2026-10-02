//! Wire routing: how a harness is dressed, not a straight line through parts.
//!
//! A wire's centreline is found by a grid search over the case floor plan at
//! the height the wire runs: inside the cavity, round every obstacle that
//! rises to that height, preferring few bends — a dressed wire runs straight
//! and turns square. Its cores are then true parallel offsets of that one
//! centreline, so they keep their sides through every bend and never cross,
//! and each core ends on its own point (its pad), approached square to the
//! row the points make.
//!
//! Pure geometry in, points out: `hardware.rs` decides what the obstacles and
//! ends are; this decides nothing about parts.

use std::collections::BinaryHeap;

pub type P = (f64, f64);

/// Something a wire must go round, at the height it runs: an axis-aligned
/// rectangle (a socket, a tall part) or a disc (a screw boss).
#[derive(Clone, Copy, Debug)]
pub enum Obstacle {
    Rect { centre: P, size: P },
    Disc { centre: P, r: f64 },
}

impl Obstacle {
    fn blocks(&self, p: P, margin: f64) -> bool {
        match *self {
            Obstacle::Rect { centre, size } => {
                (p.0 - centre.0).abs() < size.0 / 2.0 + margin
                    && (p.1 - centre.1).abs() < size.1 / 2.0 + margin
            }
            Obstacle::Disc { centre, r } => (p.0 - centre.0).hypot(p.1 - centre.1) < r + margin,
        }
    }
}

/// The room a wire has: the cavity outline, how far to keep off its walls and
/// off every obstacle, the grid step.
pub struct Floor<'a> {
    pub cavity: &'a [P],
    pub wall_margin: f64,
    pub obstacles: &'a [Obstacle],
    pub margin: f64,
    pub step: f64,
}

impl Floor<'_> {
    fn free(&self, p: P) -> bool {
        if !inside(self.cavity, p) || edge_distance(self.cavity, p) < self.wall_margin {
            return false;
        }
        !self.obstacles.iter().any(|o| o.blocks(p, self.margin))
    }
}

fn inside(poly: &[P], p: P) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            c = !c;
        }
    }
    c
}

fn edge_distance(poly: &[P], p: P) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let (vx, vy) = (b.0 - a.0, b.1 - a.1);
            let l2 = (vx * vx + vy * vy).max(1e-12);
            let t = (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / l2).clamp(0.0, 1.0);
            (p.0 - a.0 - t * vx).hypot(p.1 - a.1 - t * vy)
        })
        .fold(f64::MAX, f64::min)
}

/// A grid cell and the direction the search entered it from.
type State = ((i64, i64), u8);

#[derive(PartialEq)]
struct Node {
    cost: f64,
    at: (i64, i64),
    dir: u8,
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        o.cost
            .partial_cmp(&self.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

/// The centreline from `a` to `b`: square turns only, as few as the room
/// allows, clear of the walls and every obstacle. Its corners, ends included.
/// `None` when no clear way exists — the caller says what is in the way.
pub fn route(floor: &Floor, a: P, b: P) -> Option<Vec<P>> {
    let s = floor.step;
    let to_cell = |p: P| ((p.0 / s).round() as i64, (p.1 / s).round() as i64);
    let to_pt = |c: (i64, i64)| (c.0 as f64 * s, c.1 as f64 * s);
    // An end inside an obstacle (a pad on a board, under a part's edge) is
    // allowed: the search starts from the nearest free cell to it.
    let nearest_free = |p: P| -> Option<(i64, i64)> {
        let c = to_cell(p);
        for r in 0..60_i64 {
            let mut best: Option<((i64, i64), f64)> = None;
            for dx in -r..=r {
                for dy in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let q = (c.0 + dx, c.1 + dy);
                    if floor.free(to_pt(q)) {
                        let d = (to_pt(q).0 - p.0).hypot(to_pt(q).1 - p.1);
                        if best.is_none_or(|b| d < b.1) {
                            best = Some((q, d));
                        }
                    }
                }
            }
            if let Some((q, _)) = best {
                return Some(q);
            }
        }
        None
    };
    let (start, goal) = (nearest_free(a)?, nearest_free(b)?);
    const DIRS: [(i64, i64); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let bend = 6.0; // a bend costs this many steps: few, square turns
    let mut best: std::collections::HashMap<State, f64> = Default::default();
    let mut prev: std::collections::HashMap<State, State> = Default::default();
    let mut heap = BinaryHeap::new();
    let h = |c: (i64, i64)| ((c.0 - goal.0).abs() + (c.1 - goal.1).abs()) as f64;
    for d in 0..4u8 {
        best.insert((start, d), 0.0);
        heap.push(Node {
            cost: h(start),
            at: start,
            dir: d,
        });
    }
    let mut end: Option<State> = None;
    let mut expanded = 0usize;
    while let Some(Node { at, dir, .. }) = heap.pop() {
        expanded += 1;
        if expanded > 400_000 {
            break;
        }
        if at == goal {
            end = Some((at, dir));
            break;
        }
        let g = best[&(at, dir)];
        for (nd, (dx, dy)) in DIRS.iter().enumerate() {
            let nd = nd as u8;
            let nxt = (at.0 + dx, at.1 + dy);
            if nxt != goal && !floor.free(to_pt(nxt)) {
                continue;
            }
            let ng = g + 1.0 + if nd != dir { bend } else { 0.0 };
            if best.get(&(nxt, nd)).is_none_or(|&c| ng < c - 1e-9) {
                best.insert((nxt, nd), ng);
                prev.insert((nxt, nd), (at, dir));
                heap.push(Node {
                    cost: ng + h(nxt),
                    at: nxt,
                    dir: nd,
                });
            }
        }
    }
    let mut k = end?;
    let mut cells = vec![k.0];
    while let Some(&p) = prev.get(&k) {
        cells.push(p.0);
        k = p;
    }
    cells.reverse();
    cells.dedup();
    // Corners only: a run of cells in one direction is one segment.
    let raw: Vec<P> = cells.iter().map(|&c| to_pt(c)).collect();
    let mut mid = corners(&raw);
    // Slide the first and last runs onto the true ends when the room allows,
    // so an end off the grid is met head-on, not by a half-millimetre jog.
    let run_free = |p: P, q: P, near: P| {
        let n = ((q.0 - p.0).hypot(q.1 - p.1) / s).ceil().max(1.0) as usize;
        (0..=n).all(|i| {
            let t = i as f64 / n as f64;
            let x = (p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t);
            (x.0 - near.0).hypot(x.1 - near.1) < 1.5 * s || floor.free(x)
        })
    };
    if mid.len() >= 2 {
        let (m0, m1) = (mid[0], mid[1]);
        let (n0, n1) = if (m0.1 - m1.1).abs() < 1e-9 {
            ((m0.0, a.1), (m1.0, a.1))
        } else {
            ((a.0, m0.1), (a.0, m1.1))
        };
        if run_free(n0, n1, a) {
            mid[0] = n0;
            mid[1] = n1;
        }
        let k = mid.len();
        let (m0, m1) = (mid[k - 2], mid[k - 1]);
        let (n0, n1) = if (m0.1 - m1.1).abs() < 1e-9 {
            ((m0.0, b.1), (m1.0, b.1))
        } else {
            ((b.0, m0.1), (b.0, m1.1))
        };
        if (k > 2 || (n0 == mid[0])) && run_free(n0, n1, b) {
            mid[k - 2] = n0;
            mid[k - 1] = n1;
        }
    }
    let mut pts: Vec<P> = vec![a];
    pts.extend(mid);
    pts.push(b);
    Some(squared(&pts))
}

fn corners(raw: &[P]) -> Vec<P> {
    if raw.len() < 3 {
        return raw.to_vec();
    }
    let mut out = vec![raw[0]];
    for w in raw.windows(3) {
        let d1 = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let d2 = (w[2].0 - w[1].0, w[2].1 - w[1].1);
        if (d1.0 * d2.1 - d1.1 * d2.0).abs() > 1e-9 {
            out.push(w[1]);
        }
    }
    out.push(*raw.last().unwrap());
    out
}

/// Every segment along x or y: an end off the grid joins it with one square
/// jog, never a diagonal. Collinear points are merged.
fn squared(pts: &[P]) -> Vec<P> {
    let mut out: Vec<P> = vec![pts[0]];
    for &p in &pts[1..] {
        let q = *out.last().unwrap();
        if (p.0 - q.0).abs() > 1e-6 && (p.1 - q.1).abs() > 1e-6 {
            out.push((p.0, q.1));
        }
        out.push(p);
    }
    out.dedup_by(|b, a| (b.0 - a.0).hypot(b.1 - a.1) < 1e-6);
    let mut merged: Vec<P> = Vec::new();
    for p in out {
        if merged.len() >= 2 {
            let (a, b) = (merged[merged.len() - 2], merged[merged.len() - 1]);
            let cross = (b.0 - a.0) * (p.1 - b.1) - (b.1 - a.1) * (p.0 - b.0);
            if cross.abs() < 1e-9 {
                merged.pop();
            }
        }
        merged.push(p);
    }
    merged
}

/// The cores of a multi-core wire: each the centreline offset sideways by
/// its place in the bundle (`spacing` apart, centred), mitred at every
/// corner — so a core on the left of the bundle stays on the left through
/// every bend, and no two cross. Core 0 is the leftmost, looking along the wire.
pub fn cores(centre: &[P], n: usize, spacing: f64) -> Vec<Vec<P>> {
    let offs: Vec<f64> = (0..n)
        .map(|k| (k as f64 - (n as f64 - 1.0) / 2.0) * spacing)
        .collect();
    let normal = |a: P, b: P| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l = dx.hypot(dy).max(1e-12);
        (-dy / l, dx / l) // left of the direction of travel
    };
    offs.iter()
        .map(|&o| {
            let m = centre.len();
            (0..m)
                .map(|i| {
                    let n_in = if i > 0 {
                        Some(normal(centre[i - 1], centre[i]))
                    } else {
                        None
                    };
                    let n_out = if i + 1 < m {
                        Some(normal(centre[i], centre[i + 1]))
                    } else {
                        None
                    };
                    let nv = match (n_in, n_out) {
                        (Some(a), Some(b)) => {
                            // Mitre: the bisector, lengthened so each side keeps its distance.
                            let (sx, sy) = (a.0 + b.0, a.1 + b.1);
                            let l = sx.hypot(sy);
                            if l < 1e-9 {
                                a
                            } else {
                                let (ux, uy) = (sx / l, sy / l);
                                let cos = ux * a.0 + uy * a.1;
                                (ux / cos.max(0.2), uy / cos.max(0.2))
                            }
                        }
                        (Some(a), None) | (None, Some(a)) => a,
                        (None, None) => (0.0, 0.0),
                    };
                    (centre[i].0 + nv.0 * o, centre[i].1 + nv.1 * o)
                })
                .collect()
        })
        .collect()
}

/// Which side of the bundle each of `points` is on, arriving at them along
/// `travel`: their order, leftmost first. Assign core k to `order[k]` and the
/// cores reach their points without crossing.
pub fn left_to_right(points: &[P], travel: P) -> Vec<usize> {
    let left = (-travel.1, travel.0);
    let mut idx: Vec<usize> = (0..points.len()).collect();
    idx.sort_by(|&a, &b| {
        let pa = points[a].0 * left.0 + points[a].1 * left.1;
        let pb = points[b].0 * left.0 + points[b].1 * left.1;
        pb.partial_cmp(&pa).unwrap()
    });
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(n: f64) -> Vec<P> {
        vec![(0.0, 0.0), (n, 0.0), (n, n), (0.0, n)]
    }

    fn crosses(a: &[P], b: &[P]) -> bool {
        let seg = |p: P, q: P, r: P, s: P| {
            let d = |a: P, b: P, c: P| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
            let (d1, d2, d3, d4) = (d(p, q, r), d(p, q, s), d(r, s, p), d(r, s, q));
            d1 * d2 < -1e-9 && d3 * d4 < -1e-9
        };
        a.windows(2)
            .any(|x| b.windows(2).any(|y| seg(x[0], x[1], y[0], y[1])))
    }

    #[test]
    fn a_route_goes_round_an_obstacle_with_square_turns() {
        let cav = square(60.0);
        let obs = [Obstacle::Rect {
            centre: (30.0, 30.0),
            size: (20.0, 40.0),
        }];
        let f = Floor {
            cavity: &cav,
            wall_margin: 2.0,
            obstacles: &obs,
            margin: 1.0,
            step: 1.0,
        };
        let path = route(&f, (10.0, 30.0), (50.0, 30.0)).expect("a way round");
        for w in path.windows(2) {
            assert!(
                (w[0].0 - w[1].0).abs() < 1e-6 || (w[0].1 - w[1].1).abs() < 1e-6,
                "square: {path:?}"
            );
            for k in 1..20 {
                let t = k as f64 / 20.0;
                let p = (
                    w[0].0 + (w[1].0 - w[0].0) * t,
                    w[0].1 + (w[1].1 - w[0].1) * t,
                );
                assert!(
                    !obs[0].blocks(p, 0.0),
                    "through the obstacle at {p:?}: {path:?}"
                );
            }
        }
        // Round it is two bends each side at most: it does not wander.
        assert!(path.len() <= 6, "{path:?}");
    }

    #[test]
    fn no_way_through_is_none_not_a_line_through_the_part() {
        let cav = square(60.0);
        let obs = [Obstacle::Rect {
            centre: (30.0, 30.0),
            size: (8.0, 80.0),
        }];
        let f = Floor {
            cavity: &cav,
            wall_margin: 2.0,
            obstacles: &obs,
            margin: 1.0,
            step: 1.0,
        };
        assert!(route(&f, (10.0, 30.0), (50.0, 30.0)).is_none());
    }

    #[test]
    fn cores_keep_their_sides_through_every_bend() {
        let centre = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
        let cs = cores(&centre, 2, 1.3);
        assert!(!crosses(&cs[0], &cs[1]), "{cs:?}");
        for (a, b) in cs[0].iter().zip(&cs[1]) {
            assert!(
                ((a.0 - b.0).hypot(a.1 - b.1) - 1.3).abs() < 0.6,
                "spacing kept: {a:?} {b:?}"
            );
        }
    }

    #[test]
    fn points_are_ordered_by_the_side_they_are_on() {
        // Travelling +x, left is +y: the higher point is first.
        assert_eq!(
            left_to_right(&[(0.0, -2.0), (0.0, 2.0)], (1.0, 0.0)),
            vec![1, 0]
        );
    }
}
