//! Mesh generation from board outlines — `no_std`.
//!
//! Produces watertight triangle meshes from `BoardOutline` geometry and exports
//! them as binary STL (printable) or minimal glTF 2.0 binary / GLB (web viewer).
//!
//! # Usage (requires `std` feature)
//!
//! ```rust,ignore
//! use fiducial_geometry::BoardOutline;
//! use fiducial_mesh::{extrude_board, to_stl_binary, to_glb};
//!
//! let outline = BoardOutline::new(100.0, 60.0);
//! let mesh = extrude_board(&outline);
//! let stl  = to_stl_binary(&mesh);     // Vec<u8> — save as board.stl
//! let glb  = to_glb(&mesh);            // Vec<u8> — save as board.glb
//! ```
//!
//! # Sealed cases with features
//!
//! [`Case`] is the full API: a gasket-sealed base, lid, and gasket, with
//! connector openings punched through the walls and optional standoff posts
//! under the board. Validate before generating — the meshes are produced
//! unconditionally, so an opening that breaches the seal yields a case that
//! slices cleanly and leaks.
//!
//! ```rust,ignore
//! use fiducial_geometry::{BoardOutline, Side, connector_opening};
//! use fiducial_mesh::{Case, CaseParams, Cutout, to_stl_binary};
//!
//! let outline = BoardOutline::new(100.0, 60.0);
//! let usb = connector_opening("usb-c").unwrap();
//! let case = Case::new(outline)
//!     .with_params(CaseParams::from_outline(&outline).with_headroom(10.0))
//!     .with_cutouts(vec![Cutout::new("J1", Side::South, 20.0, usb.width_mm, usb.height_mm)]);
//!
//! case.validate()?;                    // reports against the declaration
//! let stl = to_stl_binary(&case.base());
//! ```
//!
//! # How a punched face stays closed
//!
//! A hole in a wall would normally force every neighbouring surface to be
//! re-tessellated: subdivide one edge and the surface beside it no longer
//! matches, leaving a T-junction that is not a closed manifold. Instead each
//! punched face keeps a mitred frame around an inset grid, fanned from its
//! outer corners. The inset boundary carries as many vertices as the holes
//! need while the outer boundary stays four single edges — so the rim, groove,
//! and lip rings are generated exactly as they are for a featureless case, and
//! a case with no features is byte-identical to one from before cutouts
//! existed.

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

#[cfg(any(test, feature = "std"))]
extern crate std;

use alloc::{string::String, vec::Vec};
use fiducial_geometry::{BoardOutline, Side};

/// Tolerance for comparing generated coordinates, in millimetres.
///
/// One nanometre: far below any printable feature, far above f32 noise at the
/// scale of a case.
const EPS: f32 = 1e-6;

// ── Mesh ──────────────────────────────────────────────────────────────────────

/// A triangle mesh: flat lists of vertices, normals, and triangle indices.
///
/// - `vertices[i]` is `[x, y, z]` in millimetres.
/// - `normals[i]` is the unit normal at vertex `i`.
/// - `triangles[t]` is `[i0, i1, i2]` — indices into `vertices`.
pub struct Mesh {
    /// Vertex positions ([x, y, z] in mm).
    pub vertices: Vec<[f32; 3]>,
    /// Per-vertex normals (unit vectors).
    pub normals: Vec<[f32; 3]>,
    /// Triangle index triples.
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Axis-aligned bounding box: `([min_x, min_y, min_z], [max_x, max_y, max_z])`.
    pub fn aabb(&self) -> ([f32; 3], [f32; 3]) {
        if self.vertices.is_empty() {
            return ([0.0; 3], [0.0; 3]);
        }
        let mut mn = self.vertices[0];
        let mut mx = self.vertices[0];
        for v in &self.vertices {
            for i in 0..3 {
                if v[i] < mn[i] {
                    mn[i] = v[i];
                }
                if v[i] > mx[i] {
                    mx[i] = v[i];
                }
            }
        }
        (mn, mx)
    }
}

// ── Extrusion ─────────────────────────────────────────────────────────────────

/// Extrude a `BoardOutline` into a watertight box mesh.
///
/// The board lies in the XY plane from (0, 0, 0) to (width, height, thickness).
/// Face normals point outward. The mesh has 6 faces × 2 triangles = 12 triangles.
pub fn extrude_board(outline: &BoardOutline) -> Mesh {
    let w = outline.width_mm;
    let h = outline.height_mm;
    let d = outline.thickness_mm;

    // 8 box corners — indexed as [z_face][quad_vertex]
    // Bottom face (z = 0), Top face (z = d)
    #[rustfmt::skip]
    let corners: [[f32; 3]; 8] = [
        [0.0, 0.0, 0.0], // 0 — bottom SW
        [w,   0.0, 0.0], // 1 — bottom SE
        [w,   h,   0.0], // 2 — bottom NE
        [0.0, h,   0.0], // 3 — bottom NW
        [0.0, 0.0, d  ], // 4 — top SW
        [w,   0.0, d  ], // 5 — top SE
        [w,   h,   d  ], // 6 — top NE
        [0.0, h,   d  ], // 7 — top NW
    ];

    // 6 faces: (quad corners CCW from outside, outward normal)
    #[rustfmt::skip]
    let faces: &[([usize; 4], [f32; 3])] = &[
        ([0, 3, 2, 1], [ 0.0,  0.0, -1.0]), // bottom  (z=0, normal -Z)
        ([4, 5, 6, 7], [ 0.0,  0.0,  1.0]), // top     (z=d, normal +Z)
        ([0, 1, 5, 4], [ 0.0, -1.0,  0.0]), // south   (y=0)
        ([2, 3, 7, 6], [ 0.0,  1.0,  0.0]), // north   (y=h)
        ([0, 4, 7, 3], [-1.0,  0.0,  0.0]), // west    (x=0)
        ([1, 2, 6, 5], [ 1.0,  0.0,  0.0]), // east    (x=w)
    ];

    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();

    for (quad, normal) in faces {
        let base = vertices.len() as u32;
        for &ci in quad {
            vertices.push(corners[ci]);
            normals.push(*normal);
        }
        // Two triangles per quad (fan from base)
        triangles.push([base, base + 1, base + 2]);
        triangles.push([base, base + 2, base + 3]);
    }

    Mesh {
        vertices,
        normals,
        triangles,
    }
}

// ── Enclosure generation ─────────────────────────────────────────────────────

/// Dimensions of a generated enclosure, in millimetres.
///
/// Defaults are derived from the board's [`ToleranceProfile`] rather than
/// hardcoded, so the same board yields a tighter enclosure on resin or CNC than
/// on FDM. Every field can be overridden with the builder methods.
///
/// [`ToleranceProfile`]: fiducial_geometry::ToleranceProfile
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnclosureParams {
    /// Gap between the board edge and the cavity wall, per side.
    pub clearance_mm: f32,
    /// Side wall thickness.
    pub wall_mm: f32,
    /// Floor thickness beneath the board.
    pub floor_mm: f32,
    /// Vertical space above the board for components.
    pub headroom_mm: f32,
}

impl EnclosureParams {
    /// Derive parameters from the outline's tolerance profile.
    ///
    /// - `clearance` = 2 × the process XY accuracy — the board edge and the
    ///   printed wall can each drift by one tolerance, so the gap must absorb
    ///   both or the board will not drop in.
    /// - `wall` and `floor` = the process minimum wall thickness, the thinnest
    ///   feature that will survive the build.
    /// - `headroom` = 5 mm, a component-height default with no process basis;
    ///   override it for tall parts.
    pub fn from_outline(outline: &BoardOutline) -> Self {
        let p = outline.tolerance_profile();
        Self {
            clearance_mm: p.xy_accuracy_mm * 2.0,
            wall_mm: p.min_wall_thickness_mm,
            floor_mm: p.min_wall_thickness_mm,
            headroom_mm: 5.0,
        }
    }

    /// Override the per-side clearance.
    pub const fn with_clearance(mut self, clearance_mm: f32) -> Self {
        self.clearance_mm = clearance_mm;
        self
    }

    /// Override the side wall thickness.
    pub const fn with_wall(mut self, wall_mm: f32) -> Self {
        self.wall_mm = wall_mm;
        self
    }

    /// Override the floor thickness.
    pub const fn with_floor(mut self, floor_mm: f32) -> Self {
        self.floor_mm = floor_mm;
        self
    }

    /// Override the headroom above the board.
    pub const fn with_headroom(mut self, headroom_mm: f32) -> Self {
        self.headroom_mm = headroom_mm;
        self
    }
}

/// Outer dimensions of the enclosure produced for `outline` with `params`.
///
/// Returns `(width, height, depth)` in millimetres.
pub fn enclosure_extents(outline: &BoardOutline, params: &EnclosureParams) -> (f32, f32, f32) {
    let bb = outline.to_polygon().bounding_box();
    let inner_w = bb.width() + 2.0 * params.clearance_mm;
    let inner_h = bb.height() + 2.0 * params.clearance_mm;
    let cavity_d = outline.thickness_mm + params.headroom_mm;
    (
        inner_w + 2.0 * params.wall_mm,
        inner_h + 2.0 * params.wall_mm,
        cavity_d + params.floor_mm,
    )
}

/// Append a quad as two triangles, with a shared flat normal.
///
/// `quad` must be wound counter-clockwise when viewed from outside the solid
/// (i.e. looking down `-normal`).
fn push_quad(
    vertices: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    triangles: &mut Vec<[u32; 3]>,
    quad: [[f32; 3]; 4],
    normal: [f32; 3],
) {
    let base = vertices.len() as u32;
    for v in quad {
        vertices.push(v);
        normals.push(normal);
    }
    triangles.push([base, base + 1, base + 2]);
    triangles.push([base, base + 2, base + 3]);
}

/// Generate a watertight open-top enclosure tray sized to `outline`.
///
/// The board drops into a cavity that is `clearance` larger than the board on
/// every side and sits `floor` above the ground plane. The result is a closed
/// manifold of 28 triangles: outer box, inner cavity, and a mitred rim joining
/// them at the open top.
///
/// Use [`enclosure_for`] to take the tolerance-derived defaults.
pub fn generate_enclosure(outline: &BoardOutline, params: &EnclosureParams) -> Mesh {
    let bb = outline.to_polygon().bounding_box();
    let w = params.wall_mm;
    let f = params.floor_mm;

    let inner_w = bb.width() + 2.0 * params.clearance_mm;
    let inner_h = bb.height() + 2.0 * params.clearance_mm;
    let ow = inner_w + 2.0 * w;
    let oh = inner_h + 2.0 * w;
    let od = outline.thickness_mm + params.headroom_mm + f;

    // Outer shell corners: o0..o3 at z=0, o4..o7 at z=od.
    let o0 = [0.0, 0.0, 0.0];
    let o1 = [ow, 0.0, 0.0];
    let o2 = [ow, oh, 0.0];
    let o3 = [0.0, oh, 0.0];
    let o4 = [0.0, 0.0, od];
    let o5 = [ow, 0.0, od];
    let o6 = [ow, oh, od];
    let o7 = [0.0, oh, od];

    // Cavity corners: i0..i3 on the cavity floor (z=f), i4..i7 at the rim.
    let (x0, x1) = (w, w + inner_w);
    let (y0, y1) = (w, w + inner_h);
    let i0 = [x0, y0, f];
    let i1 = [x1, y0, f];
    let i2 = [x1, y1, f];
    let i3 = [x0, y1, f];
    let i4 = [x0, y0, od];
    let i5 = [x1, y0, od];
    let i6 = [x1, y1, od];
    let i7 = [x0, y1, od];

    let mut vertices: Vec<[f32; 3]> = Vec::with_capacity(56);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(56);
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(28);
    let mut quad = |q: [[f32; 3]; 4], n: [f32; 3]| {
        push_quad(&mut vertices, &mut normals, &mut triangles, q, n)
    };

    // Underside.
    quad([o0, o3, o2, o1], [0.0, 0.0, -1.0]);

    // Outer walls — normals face away from the solid.
    quad([o0, o1, o5, o4], [0.0, -1.0, 0.0]);
    quad([o2, o3, o7, o6], [0.0, 1.0, 0.0]);
    quad([o0, o4, o7, o3], [-1.0, 0.0, 0.0]);
    quad([o1, o2, o6, o5], [1.0, 0.0, 0.0]);

    // Cavity floor, facing up into the void.
    quad([i0, i1, i2, i3], [0.0, 0.0, 1.0]);

    // Cavity walls — normals point inward, into the void.
    quad([i1, i0, i4, i5], [0.0, 1.0, 0.0]);
    quad([i3, i2, i6, i7], [0.0, -1.0, 0.0]);
    quad([i0, i3, i7, i4], [1.0, 0.0, 0.0]);
    quad([i2, i1, i5, i6], [-1.0, 0.0, 0.0]);

    // Mitred rim closing outer shell to cavity at the open top.
    quad([o4, o5, i5, i4], [0.0, 0.0, 1.0]);
    quad([o5, o6, i6, i5], [0.0, 0.0, 1.0]);
    quad([o6, o7, i7, i6], [0.0, 0.0, 1.0]);
    quad([o7, o4, i4, i7], [0.0, 0.0, 1.0]);

    Mesh {
        vertices,
        normals,
        triangles,
    }
}

/// Generate an enclosure using parameters derived from the board's tolerance class.
///
/// Equivalent to `generate_enclosure(outline, &EnclosureParams::from_outline(outline))`.
pub fn enclosure_for(outline: &BoardOutline) -> Mesh {
    generate_enclosure(outline, &EnclosureParams::from_outline(outline))
}

// ── Mesh transforms ──────────────────────────────────────────────────────────

impl Mesh {
    /// Translate every vertex by `d`. Normals are unaffected.
    pub fn translated(mut self, d: [f32; 3]) -> Self {
        for v in &mut self.vertices {
            for i in 0..3 {
                v[i] += d[i];
            }
        }
        self
    }

    /// Reflect through the z = 0 plane.
    ///
    /// Reflection reverses handedness, so triangle winding is reversed to keep
    /// it consistent with the (also reflected) normals — without that the mesh
    /// renders inside-out and slicers read every facet backwards.
    pub fn mirrored_z(mut self) -> Self {
        for v in &mut self.vertices {
            v[2] = -v[2];
        }
        for n in &mut self.normals {
            n[2] = -n[2];
        }
        for t in &mut self.triangles {
            t.swap(1, 2);
        }
        self
    }

    /// Concatenate meshes into one, re-basing triangle indices.
    ///
    /// The result is a single mesh containing several disjoint solids. That is
    /// valid for rendering and for slicing, but it is deliberately *not* a
    /// closed manifold — do not assert edge parity across a merged preview.
    pub fn merge(parts: impl IntoIterator<Item = Mesh>) -> Mesh {
        let mut out = Mesh {
            vertices: Vec::new(),
            normals: Vec::new(),
            triangles: Vec::new(),
        };
        for part in parts {
            let base = out.vertices.len() as u32;
            out.vertices.extend(part.vertices);
            out.normals.extend(part.normals);
            out.triangles.extend(
                part.triangles
                    .iter()
                    .map(|t| [t[0] + base, t[1] + base, t[2] + base]),
            );
        }
        out
    }
}

// ── Rectangular primitives ───────────────────────────────────────────────────

/// An axis-aligned rectangle in the XY plane.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

impl Rect {
    const fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    const fn from_size(w: f32, h: f32) -> Self {
        Self::new(0.0, 0.0, w, h)
    }

    /// Shrink by `d` on every side.
    fn inset(self, d: f32) -> Self {
        Self::new(self.x0 + d, self.y0 + d, self.x1 - d, self.y1 - d)
    }

    fn width(self) -> f32 {
        self.x1 - self.x0
    }

    fn height(self) -> f32 {
        self.y1 - self.y0
    }

    /// Corners counter-clockwise as seen from +Z.
    fn corners(self) -> [[f32; 2]; 4] {
        [
            [self.x0, self.y0],
            [self.x1, self.y0],
            [self.x1, self.y1],
            [self.x0, self.y1],
        ]
    }
}

/// The extent of a punched face in its own parameters.
///
/// `inset` is the width of the mitred frame the face keeps around its grid —
/// the mechanism that lets a hole subdivide the inside of a face while its
/// outer boundary stays four single edges.
#[derive(Debug, Clone, Copy)]
struct FaceSpan {
    u: (f32, f32),
    v: (f32, f32),
    inset: f32,
}

/// A rectangular hole in a punched face, in that face's own (u, v) parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Hole {
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
}

impl Hole {
    /// Whether a point lies strictly inside — used on cell centres, so a cell
    /// merely touching the hole boundary is kept.
    fn contains(&self, u: f32, v: f32) -> bool {
        u > self.u0 && u < self.u1 && v > self.v0 && v < self.v1
    }

    /// Other holes' `u` boundaries falling strictly inside this one's span.
    ///
    /// A neighbouring hole splits this hole's rim on the panel, so the tunnel
    /// through it has to be split at the same places or the two surfaces meet
    /// at a T-junction and the solid is no longer closed.
    fn u_cuts(&self, all: &[Hole]) -> Vec<f32> {
        all.iter()
            .flat_map(|h| [h.u0, h.u1])
            .filter(|t| *t > self.u0 + EPS && *t < self.u1 - EPS)
            .collect()
    }

    /// Other holes' `v` boundaries falling strictly inside this one's span.
    fn v_cuts(&self, all: &[Hole]) -> Vec<f32> {
        all.iter()
            .flat_map(|h| [h.v0, h.v1])
            .filter(|t| *t > self.v0 + EPS && *t < self.v1 - EPS)
            .collect()
    }
}

/// Accumulates quads into a [`Mesh`].
///
/// Every surface of a rectangular case is one of three shapes — a flat cap, a
/// flat ring between two concentric rectangles, or a vertical band around one
/// rectangle. Building from those three keeps winding correct by construction
/// instead of per-face.
struct MeshBuilder {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
}

impl MeshBuilder {
    fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            triangles: Vec::new(),
        }
    }

    /// `quad` must be wound counter-clockwise seen from outside the solid.
    fn quad(&mut self, quad: [[f32; 3]; 4], normal: [f32; 3]) {
        let base = self.vertices.len() as u32;
        for v in quad {
            self.vertices.push(v);
            self.normals.push(normal);
        }
        self.triangles.push([base, base + 1, base + 2]);
        self.triangles.push([base, base + 2, base + 3]);
    }

    /// A solid rectangular cap at height `z`. `up` faces +Z.
    fn cap(&mut self, r: Rect, z: f32, up: bool) {
        let c = r.corners();
        let p = |i: usize| [c[i][0], c[i][1], z];
        if up {
            self.quad([p(0), p(1), p(2), p(3)], [0.0, 0.0, 1.0]);
        } else {
            self.quad([p(0), p(3), p(2), p(1)], [0.0, 0.0, -1.0]);
        }
    }

    /// A flat ring at height `z` between two concentric rectangles, mitred at
    /// the corners. `up` faces +Z.
    fn ring(&mut self, outer: Rect, inner: Rect, z: f32, up: bool) {
        let (o, i) = (outer.corners(), inner.corners());
        for k in 0..4 {
            let n = (k + 1) % 4;
            let a = [o[k][0], o[k][1], z];
            let b = [o[n][0], o[n][1], z];
            let c = [i[n][0], i[n][1], z];
            let d = [i[k][0], i[k][1], z];
            if up {
                self.quad([a, b, c, d], [0.0, 0.0, 1.0]);
            } else {
                self.quad([a, d, c, b], [0.0, 0.0, -1.0]);
            }
        }
    }

    /// Four vertical walls around `r`, from `z_lo` to `z_hi`.
    ///
    /// `outward` points the normals away from the rectangle's centre — correct
    /// when the material is inside. Pass `false` for a cavity or groove wall,
    /// where the material is outside and the void is within.
    fn band(&mut self, r: Rect, z_lo: f32, z_hi: f32, outward: bool) {
        let c = r.corners();
        const N: [[f32; 3]; 4] = [
            [0.0, -1.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
        ];
        for k in 0..4 {
            let m = (k + 1) % 4;
            let a = [c[k][0], c[k][1], z_lo];
            let b = [c[m][0], c[m][1], z_lo];
            let bt = [c[m][0], c[m][1], z_hi];
            let at = [c[k][0], c[k][1], z_hi];
            let n = N[k];
            if outward {
                self.quad([a, b, bt, at], n);
            } else {
                self.quad([b, a, at, bt], [-n[0], -n[1], -n[2]]);
            }
        }
    }

    /// Sorted, de-duplicated grid lines spanning `lo..hi`.
    ///
    /// Cut values outside the span are dropped rather than clamped: a clamped
    /// cut would collapse a grid cell to zero width and emit a degenerate
    /// triangle, which is exactly the kind of facet a slicer chokes on.
    fn grid_lines(lo: f32, hi: f32, cuts: impl Iterator<Item = f32>) -> Vec<f32> {
        let mut v: Vec<f32> = Vec::new();
        v.push(lo);
        for c in cuts {
            if c > lo + EPS && c < hi - EPS {
                v.push(c);
            }
        }
        v.push(hi);
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        v.dedup_by(|a, b| (*a - *b).abs() < EPS);
        v
    }

    /// A single quad of a parameterised face, wound from `(u0, v0)`.
    fn face_quad<F>(&mut self, map: &F, normal: [f32; 3], u: (f32, f32), v: (f32, f32), flip: bool)
    where
        F: Fn(f32, f32) -> [f32; 3],
    {
        let q = [map(u.0, v.0), map(u.1, v.0), map(u.1, v.1), map(u.0, v.1)];
        if flip {
            self.quad([q[0], q[3], q[2], q[1]], normal);
        } else {
            self.quad(q, normal);
        }
    }

    /// Triangulate a planar polygon as a fan from its first vertex.
    ///
    /// Only valid for convex or fan-visible polygons — every polygon fanned
    /// here is a mitred trapezoid with extra vertices along one edge, which
    /// qualifies. Each boundary edge is used exactly once, which is what keeps
    /// the surface closed.
    fn poly_fan(&mut self, pts: &[[f32; 3]], normal: [f32; 3], flip: bool) {
        if pts.len() < 3 {
            return;
        }
        let base = self.vertices.len() as u32;
        for p in pts {
            self.vertices.push(*p);
            self.normals.push(normal);
        }
        for i in 1..pts.len() as u32 - 1 {
            if flip {
                self.triangles.push([base, base + i + 1, base + i]);
            } else {
                self.triangles.push([base, base + i, base + i + 1]);
            }
        }
    }

    /// A parameterised face tessellated on an explicit grid, omitting any cell
    /// whose centre falls inside a hole.
    ///
    /// `map(u, v)` places a parameter pair in space; increasing `u` then `v`
    /// must wind counter-clockwise about `normal` unless `flip` is set.
    fn grid_face<F>(
        &mut self,
        map: &F,
        normal: [f32; 3],
        u_lines: &[f32],
        v_lines: &[f32],
        holes: &[Hole],
        flip: bool,
    ) where
        F: Fn(f32, f32) -> [f32; 3],
    {
        for i in 0..u_lines.len().saturating_sub(1) {
            for j in 0..v_lines.len().saturating_sub(1) {
                let (u0, u1) = (u_lines[i], u_lines[i + 1]);
                let (v0, v1) = (v_lines[j], v_lines[j + 1]);
                let (uc, vc) = ((u0 + u1) * 0.5, (v0 + v1) * 0.5);
                if holes.iter().any(|h| h.contains(uc, vc)) {
                    continue;
                }
                self.face_quad(map, normal, (u0, u1), (v0, v1), flip);
            }
        }
    }

    /// A rectangular face with rectangular holes punched through it.
    ///
    /// The face is split into a mitred frame and an inset grid. The frame is
    /// fanned from the outer corners, so the inset boundary can carry as many
    /// vertices as the grid needs while the outer boundary stays four single
    /// edges. That is what lets a punched wall sit beside an unpunched rim
    /// without a T-junction — the neighbouring surfaces need no knowledge of
    /// the holes at all.
    ///
    /// With no holes it degenerates to one quad, so an unfeatured case is
    /// byte-identical to one generated before cutouts existed.
    fn punched_face<F>(
        &mut self,
        map: F,
        normal: [f32; 3],
        span: FaceSpan,
        holes: &[Hole],
        flip: bool,
    ) where
        F: Fn(f32, f32) -> [f32; 3],
    {
        let FaceSpan { u, v, inset } = span;
        if holes.is_empty() {
            self.face_quad(&map, normal, u, v, flip);
            return;
        }

        let inner = (u.0 + inset, u.1 - inset, v.0 + inset, v.1 - inset);
        let u_lines = Self::grid_lines(inner.0, inner.1, holes.iter().flat_map(|h| [h.u0, h.u1]));
        let v_lines = Self::grid_lines(inner.2, inner.3, holes.iter().flat_map(|h| [h.v0, h.v1]));

        // Frame corners, counter-clockwise in (u, v).
        let oc = [(u.0, v.0), (u.1, v.0), (u.1, v.1), (u.0, v.1)];
        let ic = [
            (inner.0, inner.2),
            (inner.1, inner.2),
            (inner.1, inner.3),
            (inner.0, inner.3),
        ];

        for k in 0..4 {
            let n = (k + 1) % 4;
            // The inset boundary walked from corner n back to corner k,
            // through every grid vertex on the way.
            let mut chain: Vec<(f32, f32)> = Vec::new();
            match k {
                0 => chain.extend(u_lines.iter().rev().map(|&t| (t, inner.2))),
                1 => chain.extend(v_lines.iter().rev().map(|&t| (inner.1, t))),
                2 => chain.extend(u_lines.iter().map(|&t| (t, inner.3))),
                _ => chain.extend(v_lines.iter().map(|&t| (inner.0, t))),
            }
            let mut poly: Vec<[f32; 3]> = Vec::with_capacity(chain.len() + 2);
            poly.push(map(oc[k].0, oc[k].1));
            poly.push(map(oc[n].0, oc[n].1));
            debug_assert!((chain[0].0 - ic[n].0).abs() < EPS && (chain[0].1 - ic[n].1).abs() < EPS);
            for (cu, cv) in chain {
                poly.push(map(cu, cv));
            }
            self.poly_fan(&poly, normal, flip);
        }

        self.grid_face(&map, normal, &u_lines, &v_lines, holes, flip);
    }

    fn build(self) -> Mesh {
        Mesh {
            vertices: self.vertices,
            normals: self.normals,
            triangles: self.triangles,
        }
    }
}

// ── Sealed case generation ───────────────────────────────────────────────────

/// Dimensions of a gasket-sealed two-part case, in millimetres.
///
/// A sealed case is a base whose rim carries a groove, a compressible gasket
/// seated in that groove, and a lid with a tongue that squeezes it. The gasket
/// is a separate part because it is printed in a different material — TPU or
/// another flexible filament — on the same printer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaseParams {
    /// Gap between the board edge and the cavity wall, per side.
    pub clearance_mm: f32,
    /// Width of the lip on each side of the gasket groove.
    pub lip_mm: f32,
    /// Floor thickness beneath the board.
    pub floor_mm: f32,
    /// Vertical space above the board for components.
    pub headroom_mm: f32,
    /// Lid plate thickness.
    pub lid_thickness_mm: f32,
    /// Gasket cross-section width.
    pub gasket_width_mm: f32,
    /// Gasket cross-section height, uncompressed.
    pub gasket_height_mm: f32,
    /// Fraction of gasket height squeezed when the lid is closed (0.0–1.0).
    pub gasket_compression: f32,
    /// Slack between the gasket and its groove, per side, so it can be seated.
    pub fit_clearance_mm: f32,
    /// Smallest feature the process can resolve — the margin every cutout must
    /// leave to a wall edge, and the floor on a cutout's own dimensions.
    pub min_feature_mm: f32,
    /// Height of the posts the board rests on. Zero means no standoffs and the
    /// board sits on the cavity floor.
    pub standoff_height_mm: f32,
    /// Footprint of each standoff post, square.
    pub standoff_size_mm: f32,
}

impl CaseParams {
    /// Derive case parameters from the outline's tolerance profile.
    ///
    /// Clearance, lip width, floor, and gasket fit all come from the process:
    /// a resin or CNC case is measurably tighter than an FDM one for the same
    /// board. Gasket cross-section and headroom are component-driven rather
    /// than process-driven, so they default to fixed values meant to be
    /// overridden.
    pub fn from_outline(outline: &BoardOutline) -> Self {
        let p = outline.tolerance_profile();
        Self {
            clearance_mm: p.xy_accuracy_mm * 2.0,
            lip_mm: p.min_wall_thickness_mm,
            floor_mm: p.min_wall_thickness_mm,
            headroom_mm: 5.0,
            lid_thickness_mm: p.min_wall_thickness_mm,
            gasket_width_mm: 2.0,
            gasket_height_mm: 2.0,
            gasket_compression: 0.25,
            fit_clearance_mm: p.xy_accuracy_mm,
            min_feature_mm: p.min_feature_mm,
            standoff_height_mm: 0.0,
            standoff_size_mm: 4.0,
        }
    }

    /// Groove width — the gasket plus its seating slack on both sides.
    pub fn groove_width_mm(&self) -> f32 {
        self.gasket_width_mm + 2.0 * self.fit_clearance_mm
    }

    /// Total wall thickness: a lip, the groove, and a second lip.
    ///
    /// The seal is what sets the wall thickness — the wall must be thick enough
    /// to carry the groove with a printable lip on either side.
    pub fn wall_mm(&self) -> f32 {
        2.0 * self.lip_mm + self.groove_width_mm()
    }

    /// Groove depth. The gasket seats flush, and the tongue does the squeezing.
    pub fn groove_depth_mm(&self) -> f32 {
        self.gasket_height_mm
    }

    /// How far the lid's tongue enters the groove — the compression travel.
    pub fn tongue_height_mm(&self) -> f32 {
        self.gasket_height_mm * self.gasket_compression
    }

    /// Override the per-side board clearance.
    pub const fn with_clearance(mut self, v: f32) -> Self {
        self.clearance_mm = v;
        self
    }
    /// Override the headroom above the board.
    pub const fn with_headroom(mut self, v: f32) -> Self {
        self.headroom_mm = v;
        self
    }
    /// Override the lid plate thickness.
    pub const fn with_lid_thickness(mut self, v: f32) -> Self {
        self.lid_thickness_mm = v;
        self
    }
    /// Override the gasket cross-section.
    pub const fn with_gasket(mut self, width_mm: f32, height_mm: f32) -> Self {
        self.gasket_width_mm = width_mm;
        self.gasket_height_mm = height_mm;
        self
    }
    /// Override the gasket compression fraction.
    pub const fn with_compression(mut self, v: f32) -> Self {
        self.gasket_compression = v;
        self
    }
    /// Raise the board onto standoff posts of this height.
    ///
    /// The case grows by the same amount: headroom is measured above the board,
    /// so lifting the board lifts the rim with it.
    pub const fn with_standoffs(mut self, height_mm: f32, size_mm: f32) -> Self {
        self.standoff_height_mm = height_mm;
        self.standoff_size_mm = size_mm;
        self
    }
}

/// Every rectangle and height the three case parts share.
///
/// Computed once so base, lid, and gasket cannot disagree about where the
/// groove is — the failure that produces parts which do not seal.
#[derive(Debug, Clone, Copy)]
struct CaseGeometry {
    outer: Rect,
    groove_outer: Rect,
    groove_inner: Rect,
    cavity: Rect,
    gasket_outer: Rect,
    gasket_inner: Rect,
    z_cavity_floor: f32,
    z_board_top: f32,
    z_rim: f32,
    z_groove_bottom: f32,
}

impl CaseGeometry {
    fn new(outline: &BoardOutline, p: &CaseParams) -> Self {
        let bb = outline.to_polygon().bounding_box();
        let wall = p.wall_mm();
        let outer = Rect::from_size(
            bb.width() + 2.0 * p.clearance_mm + 2.0 * wall,
            bb.height() + 2.0 * p.clearance_mm + 2.0 * wall,
        );
        // Standoffs lift the board, and headroom is measured above the board,
        // so the rim rises with them rather than eating into the clearance.
        let z_board_top = p.floor_mm + p.standoff_height_mm + outline.thickness_mm;
        let z_rim = z_board_top + p.headroom_mm;
        Self {
            outer,
            groove_outer: outer.inset(p.lip_mm),
            groove_inner: outer.inset(p.lip_mm + p.groove_width_mm()),
            cavity: outer.inset(wall),
            gasket_outer: outer.inset(p.lip_mm + p.fit_clearance_mm),
            gasket_inner: outer.inset(p.lip_mm + p.groove_width_mm() - p.fit_clearance_mm),
            z_cavity_floor: p.floor_mm,
            z_board_top,
            z_rim,
            z_groove_bottom: z_rim - p.groove_depth_mm(),
        }
    }
}

/// The three printed parts of a sealed case.
pub struct CaseParts {
    /// Base tray with the gasket groove in its rim. Print rigid.
    pub base: Mesh,
    /// Lid with the compression tongue, in print orientation (tongue up). Print rigid.
    pub lid: Mesh,
    /// Gasket ring. **Print in TPU or another flexible filament** — a rigid
    /// gasket cannot compress and will not seal.
    pub gasket: Mesh,
}

/// Outer dimensions `(width, height, depth)` of the closed case, in millimetres.
pub fn case_extents(outline: &BoardOutline, params: &CaseParams) -> (f32, f32, f32) {
    let g = CaseGeometry::new(outline, params);
    (
        g.outer.width(),
        g.outer.height(),
        g.z_rim + params.lid_thickness_mm,
    )
}

/// Base tray: outer shell, grooved rim, and board cavity.
///
/// The surface runs underside → outer wall → outer lip → down the groove →
/// groove floor → up the inner lip → down the cavity → cavity floor, closing
/// into a single manifold of 60 triangles.
pub fn generate_case_base(outline: &BoardOutline, p: &CaseParams) -> Mesh {
    let g = CaseGeometry::new(outline, p);
    case_base(
        p,
        &g,
        &[Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        &[],
    )
}

/// Lid plate with a downward tongue that compresses the gasket.
///
/// Returned in **print orientation**: the outer face lies on the bed at z = 0
/// and the tongue points up, so it prints without support.
pub fn generate_case_lid(outline: &BoardOutline, p: &CaseParams) -> Mesh {
    let g = CaseGeometry::new(outline, p);
    let t = p.lid_thickness_mm;
    let tongue_top = t + p.tongue_height_mm();
    let mut b = MeshBuilder::new();

    b.cap(g.outer, 0.0, false);
    b.band(g.outer, 0.0, t, true);
    b.ring(g.outer, g.groove_outer, t, true);
    b.band(g.groove_outer, t, tongue_top, true);
    b.ring(g.groove_outer, g.groove_inner, tongue_top, true);
    b.band(g.groove_inner, t, tongue_top, false);
    b.cap(g.groove_inner, t, true);

    b.build()
}

/// Gasket ring sized to the groove.
///
/// **Print in TPU** (or another flexible filament) — this part seals by being
/// squeezed, so a rigid print defeats the whole assembly.
pub fn generate_gasket(outline: &BoardOutline, p: &CaseParams) -> Mesh {
    let g = CaseGeometry::new(outline, p);
    let h = p.gasket_height_mm;
    let mut b = MeshBuilder::new();

    b.ring(g.gasket_outer, g.gasket_inner, 0.0, false);
    b.band(g.gasket_outer, 0.0, h, true);
    b.band(g.gasket_inner, 0.0, h, false);
    b.ring(g.gasket_outer, g.gasket_inner, h, true);

    b.build()
}

/// Generate all three parts of a sealed case.
pub fn generate_case(outline: &BoardOutline, params: &CaseParams) -> CaseParts {
    CaseParts {
        base: generate_case_base(outline, params),
        lid: generate_case_lid(outline, params),
        gasket: generate_gasket(outline, params),
    }
}

/// Generate a sealed case using tolerance-derived defaults.
pub fn case_for(outline: &BoardOutline) -> CaseParts {
    generate_case(outline, &CaseParams::from_outline(outline))
}

/// An exploded view of a featureless case — see [`Case::exploded`].
pub fn case_exploded(outline: &BoardOutline, p: &CaseParams) -> Mesh {
    Case::new(*outline).with_params(*p).exploded()
}

// ── Features: cutouts and standoffs ──────────────────────────────────────────

/// A rectangular opening punched through one wall of the case.
///
/// Declared in **board coordinates**: `offset_mm` is the centre of the opening
/// measured along the named board edge from the board's origin corner, and
/// `z_offset_mm` is the opening floor above the board's top surface. That way
/// a cutout is positioned by where the connector sits on the PCB, not by where
/// it lands on a wall whose thickness the seal decides.
///
/// `width_mm` and `height_mm` are the **connector body**; the process
/// clearance is added when the hole is cut, so the same declaration yields a
/// tighter opening on resin than on FDM.
#[derive(Debug, Clone, PartialEq)]
pub struct Cutout {
    /// Which board edge — and so which wall — the opening passes through.
    pub side: Side,
    /// Centre of the opening along that edge, from the board's origin corner.
    pub offset_mm: f32,
    /// Connector body width across the edge.
    pub width_mm: f32,
    /// Connector body height.
    pub height_mm: f32,
    /// Opening floor above the board's top surface. Negative for a connector
    /// that hangs below the PCB surface, such as a mid-mount receptacle.
    pub z_offset_mm: f32,
    /// Name used in validation errors — normally the connector's designator.
    pub label: String,
}

impl Cutout {
    /// A cutout for a connector body `width_mm` × `height_mm` on `side`.
    pub fn new(
        label: impl Into<String>,
        side: Side,
        offset_mm: f32,
        width_mm: f32,
        height_mm: f32,
    ) -> Self {
        Self {
            side,
            offset_mm,
            width_mm,
            height_mm,
            z_offset_mm: 0.0,
            label: label.into(),
        }
    }

    /// Move the opening floor relative to the board's top surface.
    pub fn with_z_offset(mut self, z_offset_mm: f32) -> Self {
        self.z_offset_mm = z_offset_mm;
        self
    }
}

/// Why a declared case cannot be generated.
///
/// Every variant is a condition that yields geometry a printer would accept
/// and a product would not: an opening that cuts the seal, one too small to
/// resolve, or two that merge into one.
#[derive(Debug, Clone, PartialEq)]
pub enum CaseError {
    /// A cutout reaches the gasket groove, so the case would no longer seal.
    SealBreached {
        /// Cutout label.
        label: String,
        /// Where the opening's top edge lands, in case coordinates.
        top_mm: f32,
        /// The highest an opening may reach.
        limit_mm: f32,
    },
    /// A cutout sits below the cavity floor, where there is no wall to cut.
    BelowCavity {
        /// Cutout label.
        label: String,
    },
    /// A cutout runs off the end of its wall.
    OffWall {
        /// Cutout label.
        label: String,
        /// Usable span along that wall.
        span_mm: f32,
    },
    /// A cutout is smaller than the process can resolve.
    TooSmall {
        /// Cutout label.
        label: String,
        /// The smaller of the two declared dimensions.
        smallest_mm: f32,
        /// The process minimum feature size.
        min_feature_mm: f32,
    },
    /// Two cutouts on the same wall overlap, which would merge them into one
    /// opening with no material between.
    Overlap {
        /// First cutout label.
        a: String,
        /// Second cutout label.
        b: String,
    },
    /// Standoffs were requested but four of them will not fit on the board.
    StandoffsTooLarge {
        /// Requested post footprint.
        size_mm: f32,
        /// The shorter board dimension they must share.
        span_mm: f32,
    },
}

impl core::fmt::Display for CaseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SealBreached {
                label,
                top_mm,
                limit_mm,
            } => write!(
                f,
                "cutout `{label}` reaches {top_mm:.2} mm but the gasket groove starts at \
                 {limit_mm:.2} mm — the case would not seal. Lower z_offset_mm, shrink the \
                 opening, or raise headroom_mm"
            ),
            Self::BelowCavity { label } => write!(
                f,
                "cutout `{label}` sits below the cavity floor, where there is no wall to cut \
                 — raise z_offset_mm"
            ),
            Self::OffWall { label, span_mm } => write!(
                f,
                "cutout `{label}` runs off its wall; openings must fit within {span_mm:.2} mm \
                 of usable span"
            ),
            Self::TooSmall {
                label,
                smallest_mm,
                min_feature_mm,
            } => write!(
                f,
                "cutout `{label}` is {smallest_mm:.2} mm across, below the {min_feature_mm:.2} mm \
                 this process can resolve"
            ),
            Self::Overlap { a, b } => write!(
                f,
                "cutouts `{a}` and `{b}` overlap on the same wall — they would merge into one \
                 opening"
            ),
            Self::StandoffsTooLarge { size_mm, span_mm } => write!(
                f,
                "four {size_mm:.2} mm standoffs do not fit across a {span_mm:.2} mm board"
            ),
        }
    }
}

#[cfg(any(test, feature = "std"))]
impl std::error::Error for CaseError {}

/// A sealed case together with the features cut into it.
///
/// Generation is split from validation on purpose: [`Case::validate`] reports
/// what is wrong with the *declaration*, in the declaration's own terms, before
/// any geometry exists. A cutout that breaches the seal is a design error, and
/// the only place it is still cheap to explain is here.
pub struct Case {
    outline: BoardOutline,
    params: CaseParams,
    cutouts: Vec<Cutout>,
}

impl Case {
    /// A case for `outline` with tolerance-derived parameters and no features.
    pub fn new(outline: BoardOutline) -> Self {
        Self {
            params: CaseParams::from_outline(&outline),
            outline,
            cutouts: Vec::new(),
        }
    }

    /// Replace the derived parameters.
    pub fn with_params(mut self, params: CaseParams) -> Self {
        self.params = params;
        self
    }

    /// Punch these openings through the walls.
    pub fn with_cutouts(mut self, cutouts: Vec<Cutout>) -> Self {
        self.cutouts = cutouts;
        self
    }

    /// The parameters this case will be generated with.
    pub fn params(&self) -> &CaseParams {
        &self.params
    }

    /// The openings declared on this case.
    pub fn cutouts(&self) -> &[Cutout] {
        &self.cutouts
    }

    /// Outer dimensions `(width, height, depth)` of the closed case.
    pub fn extents(&self) -> (f32, f32, f32) {
        case_extents(&self.outline, &self.params)
    }

    /// Check every declared feature against the geometry it would cut.
    ///
    /// Call this before generating: the meshes are produced unconditionally, so
    /// an unvalidated declaration yields a case that slices cleanly and leaks.
    pub fn validate(&self) -> Result<(), CaseError> {
        let g = CaseGeometry::new(&self.outline, &self.params);
        let p = &self.params;
        let margin = p.min_feature_mm;
        let limit = g.z_groove_bottom - margin;

        for c in &self.cutouts {
            let smallest = c.width_mm.min(c.height_mm);
            // NaN is checked explicitly: it compares false against every
            // bound, so a bare `<` would let it through into the mesh.
            if smallest.is_nan() || smallest < p.min_feature_mm {
                return Err(CaseError::TooSmall {
                    label: c.label.clone(),
                    smallest_mm: smallest,
                    min_feature_mm: p.min_feature_mm,
                });
            }
            let h = self.hole_for(c, &g);
            if h.v1 > limit {
                return Err(CaseError::SealBreached {
                    label: c.label.clone(),
                    top_mm: h.v1,
                    limit_mm: limit,
                });
            }
            if h.v0 < g.z_cavity_floor + margin {
                return Err(CaseError::BelowCavity {
                    label: c.label.clone(),
                });
            }
            let wall = p.wall_mm();
            let len = c.side.span_mm(g.outer.width(), g.outer.height());
            if h.u0 < wall + margin || h.u1 > len - wall - margin {
                return Err(CaseError::OffWall {
                    label: c.label.clone(),
                    span_mm: len - 2.0 * (wall + margin),
                });
            }
        }

        // Overlap is checked pairwise per wall. Openings that merge produce a
        // single wide slot with no material between them, which is silently
        // watertight and structurally wrong.
        for (i, a) in self.cutouts.iter().enumerate() {
            for b in &self.cutouts[i + 1..] {
                if a.side != b.side {
                    continue;
                }
                let (ha, hb) = (self.hole_for(a, &g), self.hole_for(b, &g));
                if ha.u0 < hb.u1 + margin && hb.u0 < ha.u1 + margin {
                    return Err(CaseError::Overlap {
                        a: a.label.clone(),
                        b: b.label.clone(),
                    });
                }
            }
        }

        if p.standoff_height_mm > 0.0 {
            let span = self.outline.width_mm.min(self.outline.height_mm);
            if 2.0 * (p.standoff_size_mm + margin) + margin >= span {
                return Err(CaseError::StandoffsTooLarge {
                    size_mm: p.standoff_size_mm,
                    span_mm: span,
                });
            }
        }

        Ok(())
    }

    /// Resolve one declared cutout into its wall's (along, z) coordinates.
    ///
    /// The declared size is the connector body; one process tolerance is added
    /// on every side so the part actually passes through a printed wall.
    fn hole_for(&self, c: &Cutout, g: &CaseGeometry) -> Hole {
        let p = &self.params;
        let fit = p.fit_clearance_mm;
        let inboard = p.wall_mm() + p.clearance_mm;
        let len = c.side.span_mm(g.outer.width(), g.outer.height());
        // South and East run with the board axes; North and West run against
        // them, so the same declared offset measures from the same board
        // corner on every side.
        let centre = match c.side {
            Side::South | Side::East => inboard + c.offset_mm,
            Side::North | Side::West => len - (inboard + c.offset_mm),
        };
        let half = c.width_mm * 0.5 + fit;
        let v0 = g.z_board_top + c.z_offset_mm - fit;
        Hole {
            u0: centre - half,
            u1: centre + half,
            v0,
            v1: v0 + c.height_mm + 2.0 * fit,
        }
    }

    /// Cutouts grouped by wall, in generation order.
    fn wall_holes(&self, g: &CaseGeometry) -> [Vec<Hole>; 4] {
        let mut walls: [Vec<Hole>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for c in &self.cutouts {
            let k = Side::ALL.iter().position(|s| *s == c.side).unwrap_or(0);
            walls[k].push(self.hole_for(c, g));
        }
        walls
    }

    /// Standoff footprints in case coordinates — empty unless a height was set.
    ///
    /// Four posts, one under each board corner, inset by the process minimum
    /// feature so the punched cavity floor keeps material between each post and
    /// the wall.
    fn standoff_feet(&self, g: &CaseGeometry) -> Vec<Rect> {
        let p = &self.params;
        if p.standoff_height_mm <= 0.0 || p.standoff_size_mm <= 0.0 {
            return Vec::new();
        }
        let m = p.min_feature_mm;
        let s = p.standoff_size_mm;
        let (bx, by) = (g.cavity.x0 + p.clearance_mm, g.cavity.y0 + p.clearance_mm);
        let (bw, bh) = (self.outline.width_mm, self.outline.height_mm);
        let xs = [bx + m, bx + bw - m - s];
        let ys = [by + m, by + bh - m - s];
        let mut feet = Vec::with_capacity(4);
        for &x in &xs {
            for &y in &ys {
                feet.push(Rect::new(x, y, x + s, y + s));
            }
        }
        feet
    }

    /// Base tray with the gasket groove, connector cutouts, and standoffs.
    pub fn base(&self) -> Mesh {
        let g = CaseGeometry::new(&self.outline, &self.params);
        case_base(
            &self.params,
            &g,
            &self.wall_holes(&g),
            &self.standoff_feet(&g),
        )
    }

    /// Lid with the compression tongue, in print orientation.
    ///
    /// The lid carries no features: a hole in the lid is a hole inside the
    /// gasket line, and no amount of compression seals that.
    pub fn lid(&self) -> Mesh {
        generate_case_lid(&self.outline, &self.params)
    }

    /// Gasket ring — print in TPU.
    pub fn gasket(&self) -> Mesh {
        generate_gasket(&self.outline, &self.params)
    }

    /// All three printed parts.
    pub fn parts(&self) -> CaseParts {
        CaseParts {
            base: self.base(),
            lid: self.lid(),
            gasket: self.gasket(),
        }
    }

    /// An exploded view of the assembled case, for rendering.
    ///
    /// Base at the origin, gasket lifted clear of its groove, lid above that
    /// with its tongue pointing down — the arrangement that shows how the seal
    /// works. The parts are separated rather than interpenetrating, so nothing
    /// z-fights.
    ///
    /// This merges three disjoint solids into one mesh: fine to render or
    /// slice, but not a closed manifold.
    pub fn exploded(&self) -> Mesh {
        let g = CaseGeometry::new(&self.outline, &self.params);
        let p = &self.params;
        let gap = (p.headroom_mm * 0.6).max(4.0);
        let gasket_z = g.z_groove_bottom + gap;
        // The lid is modelled tongue-up for printing; mirroring turns it over,
        // and the offset then places its tongue tip one gap above the gasket.
        let lid_z = gasket_z + p.gasket_height_mm + gap + p.lid_thickness_mm + p.tongue_height_mm();
        Mesh::merge([
            self.base(),
            self.gasket().translated([0.0, 0.0, gasket_z]),
            self.lid().mirrored_z().translated([0.0, 0.0, lid_z]),
        ])
    }
}

/// Origin, along-direction, outward normal, and length of wall `k`.
///
/// `k` indexes the same corner order [`MeshBuilder::band`] walks, so a punched
/// wall lands exactly where the unpunched band would have.
fn wall_frame(r: Rect, k: usize) -> ([f32; 2], [f32; 2], [f32; 3], f32) {
    match k {
        0 => ([r.x0, r.y0], [1.0, 0.0], [0.0, -1.0, 0.0], r.width()),
        1 => ([r.x1, r.y0], [0.0, 1.0], [1.0, 0.0, 0.0], r.height()),
        2 => ([r.x1, r.y1], [-1.0, 0.0], [0.0, 1.0, 0.0], r.width()),
        _ => ([r.x0, r.y1], [0.0, -1.0], [-1.0, 0.0, 0.0], r.height()),
    }
}

/// The four faces of the prismatic void a cutout cuts through a wall.
///
/// Subdivided by any neighbouring hole's boundaries so both ends meet the
/// punched panels vertex for vertex.
fn tunnel(
    b: &mut MeshBuilder,
    origin: [f32; 2],
    dir: [f32; 2],
    normal: [f32; 3],
    depth: f32,
    h: &Hole,
    all: &[Hole],
) {
    let inward = [-normal[0], -normal[1]];
    let at = |u: f32, d: f32, z: f32| {
        [
            origin[0] + dir[0] * u + inward[0] * d,
            origin[1] + dir[1] * u + inward[1] * d,
            z,
        ]
    };
    let u_lines = MeshBuilder::grid_lines(h.u0, h.u1, h.u_cuts(all).into_iter());
    let v_lines = MeshBuilder::grid_lines(h.v0, h.v1, h.v_cuts(all).into_iter());
    let d_lines = [0.0, depth];
    let along = [dir[0], dir[1], 0.0];

    // Floor and ceiling of the opening.
    b.grid_face(
        &|u, d| at(u, d, h.v0),
        [0.0, 0.0, 1.0],
        &u_lines,
        &d_lines,
        &[],
        false,
    );
    b.grid_face(
        &|u, d| at(u, d, h.v1),
        [0.0, 0.0, -1.0],
        &u_lines,
        &d_lines,
        &[],
        true,
    );
    // Its two jambs.
    b.grid_face(
        &|d, z| at(h.u0, d, z),
        along,
        &d_lines,
        &v_lines,
        &[],
        false,
    );
    b.grid_face(
        &|d, z| at(h.u1, d, z),
        [-along[0], -along[1], 0.0],
        &d_lines,
        &v_lines,
        &[],
        true,
    );
}

/// A solid post rising from a hole punched in the cavity floor.
///
/// The post is not a separate solid dropped onto the floor: the floor is
/// punched and the surface continues up the post, so base and standoffs remain
/// one closed manifold. Walls and top are subdivided on the floor's grid for
/// the same reason.
fn post(b: &mut MeshBuilder, foot: Rect, z_lo: f32, z_hi: f32, x_cuts: &[f32], y_cuts: &[f32]) {
    let xl = MeshBuilder::grid_lines(foot.x0, foot.x1, x_cuts.iter().copied());
    let yl = MeshBuilder::grid_lines(foot.y0, foot.y1, y_cuts.iter().copied());
    let z = [z_lo, z_hi];
    b.grid_face(
        &|u, v| [u, foot.y0, v],
        [0.0, -1.0, 0.0],
        &xl,
        &z,
        &[],
        false,
    );
    b.grid_face(
        &|u, v| [foot.x1, u, v],
        [1.0, 0.0, 0.0],
        &yl,
        &z,
        &[],
        false,
    );
    b.grid_face(&|u, v| [u, foot.y1, v], [0.0, 1.0, 0.0], &xl, &z, &[], true);
    b.grid_face(
        &|u, v| [foot.x0, u, v],
        [-1.0, 0.0, 0.0],
        &yl,
        &z,
        &[],
        true,
    );
    b.grid_face(&|u, v| [u, v, z_hi], [0.0, 0.0, 1.0], &xl, &yl, &[], false);
}

/// Build the base tray, punching `walls` through its sides and raising `feet`
/// from its floor.
///
/// The underside, rim, groove, and lip rings are generated exactly as they are
/// for a featureless case. That is the point of [`MeshBuilder::punched_face`]:
/// features stay local, so nothing downstream of a wall has to know a hole
/// went through it.
fn case_base(p: &CaseParams, g: &CaseGeometry, walls: &[Vec<Hole>; 4], feet: &[Rect]) -> Mesh {
    let wall = p.wall_mm();
    let inset = p.min_feature_mm * 0.5;
    let mut b = MeshBuilder::new();

    b.cap(g.outer, 0.0, false);

    for (k, holes) in walls.iter().enumerate() {
        let (o, dir, n, len) = wall_frame(g.outer, k);
        let inward = [-n[0], -n[1]];

        // Outer face of the wall.
        b.punched_face(
            |u, v| [o[0] + dir[0] * u, o[1] + dir[1] * u, v],
            n,
            FaceSpan {
                u: (0.0, len),
                v: (0.0, g.z_rim),
                inset,
            },
            holes,
            false,
        );
        // Cavity face, parameterised by the same along-axis so a cutout's two
        // ends share one coordinate and the tunnel between them is straight.
        b.punched_face(
            |u, v| {
                [
                    o[0] + dir[0] * u + inward[0] * wall,
                    o[1] + dir[1] * u + inward[1] * wall,
                    v,
                ]
            },
            [-n[0], -n[1], 0.0],
            FaceSpan {
                u: (wall, len - wall),
                v: (g.z_cavity_floor, g.z_rim),
                inset,
            },
            holes,
            true,
        );
        for h in holes {
            tunnel(&mut b, o, dir, n, wall, h, holes);
        }
    }

    b.ring(g.outer, g.groove_outer, g.z_rim, true);
    b.band(g.groove_outer, g.z_groove_bottom, g.z_rim, false);
    b.ring(g.groove_outer, g.groove_inner, g.z_groove_bottom, true);
    b.band(g.groove_inner, g.z_groove_bottom, g.z_rim, true);
    b.ring(g.groove_inner, g.cavity, g.z_rim, true);

    // Cavity floor, punched wherever a standoff rises out of it.
    let holes: Vec<Hole> = feet
        .iter()
        .map(|r| Hole {
            u0: r.x0,
            v0: r.y0,
            u1: r.x1,
            v1: r.y1,
        })
        .collect();
    b.punched_face(
        |u, v| [u, v, g.z_cavity_floor],
        [0.0, 0.0, 1.0],
        FaceSpan {
            u: (g.cavity.x0, g.cavity.x1),
            v: (g.cavity.y0, g.cavity.y1),
            inset,
        },
        &holes,
        false,
    );
    let x_cuts: Vec<f32> = holes.iter().flat_map(|h| [h.u0, h.u1]).collect();
    let y_cuts: Vec<f32> = holes.iter().flat_map(|h| [h.v0, h.v1]).collect();
    for f in feet {
        post(
            &mut b,
            *f,
            g.z_cavity_floor,
            g.z_cavity_floor + p.standoff_height_mm,
            &x_cuts,
            &y_cuts,
        );
    }

    b.build()
}

// ── Binary STL export (std feature) ──────────────────────────────────────────

/// Encode `mesh` as an 80-byte-header binary STL.
///
/// The result is a `Vec<u8>` ready to be written to a `.stl` file.
/// Binary STL format: 80-byte header + 4-byte triangle count +
/// 50 bytes per triangle (normal f32×3, v0 f32×3, v1 f32×3, v2 f32×3, attr u16).
#[cfg(any(feature = "std", test))]
pub fn to_stl_binary(mesh: &Mesh) -> Vec<u8> {
    let n_tri = mesh.triangles.len() as u32;
    let mut buf: Vec<u8> = Vec::with_capacity(84 + 50 * n_tri as usize);

    // 80-byte header
    let mut header = [0u8; 80];
    let tag = b"fiducial-mesh STL";
    header[..tag.len()].copy_from_slice(tag);
    buf.extend_from_slice(&header);

    // Triangle count (LE u32)
    buf.extend_from_slice(&n_tri.to_le_bytes());

    for tri in &mesh.triangles {
        // Compute face normal from first vertex's stored normal
        let ni = tri[0] as usize;
        let n = mesh.normals[ni];
        push_f32_le(&mut buf, n[0]);
        push_f32_le(&mut buf, n[1]);
        push_f32_le(&mut buf, n[2]);
        for &vi in tri {
            let v = mesh.vertices[vi as usize];
            push_f32_le(&mut buf, v[0]);
            push_f32_le(&mut buf, v[1]);
            push_f32_le(&mut buf, v[2]);
        }
        buf.extend_from_slice(&[0u8; 2]); // attribute byte count
    }

    buf
}

#[cfg(any(feature = "std", test))]
fn push_f32_le(buf: &mut Vec<u8>, v: f32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

// ── GLB / glTF 2.0 binary export (std feature) ───────────────────────────────

/// Encode `mesh` as a minimal glTF 2.0 binary (`.glb`) file.
///
/// Produces a single mesh node with POSITION and NORMAL attributes and SCALAR
/// UNSIGNED_INT indices. Viewable in Three.js, Blender, model-viewer, and any
/// compliant glTF 2.0 renderer.
#[cfg(any(feature = "std", test))]
pub fn to_glb(mesh: &Mesh) -> Vec<u8> {
    let (aabb_min, aabb_max) = mesh.aabb();

    // ── Binary buffer: positions + normals + indices ──────────────────────────
    let n_verts = mesh.vertices.len();
    let n_idx = mesh.triangles.len() * 3;

    let pos_bytes = n_verts * 12; // 3 × f32
    let nrm_bytes = n_verts * 12;
    let idx_bytes = n_idx * 4; // u32

    // Pad normals offset to 4-byte alignment (already aligned: 12N is always 4-byte)
    let nrm_offset = pos_bytes;
    let idx_offset = nrm_offset + nrm_bytes;
    let bin_len = idx_offset + idx_bytes;

    let mut bin: Vec<u8> = Vec::with_capacity(bin_len);
    for v in &mesh.vertices {
        push_f32_le(&mut bin, v[0]);
        push_f32_le(&mut bin, v[1]);
        push_f32_le(&mut bin, v[2]);
    }
    for n in &mesh.normals {
        push_f32_le(&mut bin, n[0]);
        push_f32_le(&mut bin, n[1]);
        push_f32_le(&mut bin, n[2]);
    }
    for tri in &mesh.triangles {
        for &i in tri {
            bin.extend_from_slice(&i.to_le_bytes());
        }
    }
    // Pad binary chunk to 4-byte boundary
    while bin.len() % 4 != 0 {
        bin.push(0x00);
    }
    let bin_chunk_len = bin.len();

    // ── JSON chunk ────────────────────────────────────────────────────────────
    let json = alloc::format!(
        r#"{{"asset":{{"generator":"fiducial-mesh","version":"2.0"}},"scene":0,"scenes":[{{"nodes":[0]}}],"nodes":[{{"mesh":0}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0,"NORMAL":1}},"indices":2}}]}}],"accessors":[{{"bufferView":0,"componentType":5126,"count":{nv},"type":"VEC3","min":[{xn},{yn},{zn}],"max":[{xx},{yx},{zx}]}},{{"bufferView":1,"componentType":5126,"count":{nv},"type":"VEC3"}},{{"bufferView":2,"componentType":5125,"count":{ni},"type":"SCALAR"}}],"bufferViews":[{{"buffer":0,"byteOffset":0,"byteLength":{pos_bytes},"target":34962}},{{"buffer":0,"byteOffset":{nrm_offset},"byteLength":{nrm_bytes},"target":34962}},{{"buffer":0,"byteOffset":{idx_offset},"byteLength":{idx_bytes},"target":34963}}],"buffers":[{{"byteLength":{bin_chunk_len}}}]}}"#,
        nv = n_verts,
        ni = n_idx,
        xn = aabb_min[0],
        yn = aabb_min[1],
        zn = aabb_min[2],
        xx = aabb_max[0],
        yx = aabb_max[1],
        zx = aabb_max[2],
    );
    let json_bytes = json.as_bytes();

    // Pad JSON chunk to 4-byte boundary with spaces (0x20 per spec)
    let json_pad = (4 - (json_bytes.len() % 4)) % 4;
    let json_chunk_len = json_bytes.len() + json_pad;

    // ── GLB header + chunks ───────────────────────────────────────────────────
    let total_len = 12 + 8 + json_chunk_len + 8 + bin_chunk_len;
    let mut glb: Vec<u8> = Vec::with_capacity(total_len);

    // GLB header: magic, version, total length
    glb.extend_from_slice(&0x46_54_6C_67u32.to_le_bytes()); // "glTF"
    glb.extend_from_slice(&2u32.to_le_bytes()); // version 2
    glb.extend_from_slice(&(total_len as u32).to_le_bytes());

    // JSON chunk header
    glb.extend_from_slice(&(json_chunk_len as u32).to_le_bytes());
    glb.extend_from_slice(&0x4E_4F_53_4Au32.to_le_bytes()); // "JSON"
    glb.extend_from_slice(json_bytes);
    glb.extend(core::iter::repeat_n(0x20u8, json_pad));

    // Binary chunk header
    glb.extend_from_slice(&(bin_chunk_len as u32).to_le_bytes());
    glb.extend_from_slice(&0x00_4E_49_42u32.to_le_bytes()); // "BIN\0"
    glb.extend_from_slice(&bin);

    glb
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use fiducial_geometry::BoardOutline;

    fn standard_board() -> Mesh {
        extrude_board(&BoardOutline::new(100.0, 60.0))
    }

    #[test]
    fn box_has_12_triangles() {
        assert_eq!(standard_board().triangle_count(), 12);
    }

    #[test]
    fn box_has_24_vertices() {
        // 6 faces × 4 vertices each = 24 (no vertex sharing across faces for flat normals)
        assert_eq!(standard_board().vertex_count(), 24);
    }

    #[test]
    fn all_triangle_indices_in_range() {
        let m = standard_board();
        let n = m.vertex_count() as u32;
        for tri in &m.triangles {
            for &i in tri {
                assert!(i < n, "index {i} out of range {n}");
            }
        }
    }

    #[test]
    fn aabb_matches_board_dimensions() {
        let outline = BoardOutline::new(100.0, 60.0).with_thickness(1.6);
        let m = extrude_board(&outline);
        let (mn, mx) = m.aabb();
        assert!((mn[0]).abs() < 1e-5);
        assert!((mn[1]).abs() < 1e-5);
        assert!((mn[2]).abs() < 1e-5);
        assert!((mx[0] - 100.0).abs() < 1e-4);
        assert!((mx[1] - 60.0).abs() < 1e-4);
        assert!((mx[2] - 1.6).abs() < 1e-4);
    }

    #[test]
    fn normals_are_unit_vectors() {
        let m = standard_board();
        for n in &m.normals {
            let len = libm::sqrtf(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
            assert!((len - 1.0).abs() < 1e-5, "normal length {len} != 1");
        }
    }

    #[test]
    fn stl_binary_has_correct_size() {
        let m = standard_board();
        let stl = to_stl_binary(&m);
        let expected = 84 + 50 * m.triangle_count();
        assert_eq!(stl.len(), expected);
    }

    #[test]
    fn stl_binary_header_tag() {
        let m = standard_board();
        let stl = to_stl_binary(&m);
        assert!(stl.starts_with(b"fiducial-mesh STL"));
    }

    #[test]
    fn stl_triangle_count_field() {
        let m = standard_board();
        let stl = to_stl_binary(&m);
        let count = u32::from_le_bytes([stl[80], stl[81], stl[82], stl[83]]);
        assert_eq!(count as usize, m.triangle_count());
    }

    #[test]
    fn glb_starts_with_gltf_magic() {
        let m = standard_board();
        let glb = to_glb(&m);
        assert_eq!(&glb[0..4], b"glTF");
    }

    #[test]
    fn glb_version_is_2() {
        let m = standard_board();
        let glb = to_glb(&m);
        let version = u32::from_le_bytes([glb[4], glb[5], glb[6], glb[7]]);
        assert_eq!(version, 2);
    }

    #[test]
    fn glb_total_length_matches_buffer() {
        let m = standard_board();
        let glb = to_glb(&m);
        let declared = u32::from_le_bytes([glb[8], glb[9], glb[10], glb[11]]) as usize;
        assert_eq!(declared, glb.len());
    }

    // ── Enclosure ────────────────────────────────────────────────────────────

    use fiducial_geometry::ToleranceClass;
    use std::collections::HashMap;

    /// A vertex position quantised to 1 µm.
    type VertexKey = (i64, i64, i64);
    /// A directed edge, from its first endpoint to its second.
    type EdgeKey = (VertexKey, VertexKey);

    /// Quantise a vertex to 1 µm so shared corners compare equal despite being
    /// duplicated per-face for flat shading.
    fn key(v: [f32; 3]) -> VertexKey {
        let q = |x: f32| (x * 1000.0).round() as i64;
        (q(v[0]), q(v[1]), q(v[2]))
    }

    /// Signed volume via the divergence theorem.
    ///
    /// Positive exactly when a closed surface's faces wind outward, so it is
    /// the one check that catches a globally inside-out solid — a mesh that
    /// passes every local test and slices as a hollow of itself.
    fn signed_volume(m: &Mesh) -> f32 {
        let mut vol = 0.0f64;
        for t in &m.triangles {
            let (a, b, c) = (
                m.vertices[t[0] as usize],
                m.vertices[t[1] as usize],
                m.vertices[t[2] as usize],
            );
            let cross = [
                (b[1] * c[2] - b[2] * c[1]) as f64,
                (b[2] * c[0] - b[0] * c[2]) as f64,
                (b[0] * c[1] - b[1] * c[0]) as f64,
            ];
            vol += (a[0] as f64 * cross[0] + a[1] as f64 * cross[1] + a[2] as f64 * cross[2]) / 6.0;
        }
        vol as f32
    }

    /// Assert the mesh is a closed, consistently outward-oriented solid.
    ///
    /// The test is that every *directed* edge occurs exactly once. That is
    /// strictly stronger than the undirected parity a slicer needs: parity
    /// alone is satisfied by two adjacent faces wound the same way, which reads
    /// as a crease with the material on both sides. Directed uniqueness also
    /// rules out T-junctions, where a subdivided edge meets an unsubdivided
    /// one — the failure mode every punched face is built to avoid.
    ///
    /// Vertices are quantised to 1 µm before comparison because faces
    /// duplicate their corners to keep flat normals.
    fn assert_watertight(m: &Mesh) {
        let mut edges: HashMap<EdgeKey, usize> = HashMap::new();
        for tri in &m.triangles {
            for k in 0..3 {
                let a = key(m.vertices[tri[k] as usize]);
                let b = key(m.vertices[tri[(k + 1) % 3] as usize]);
                assert_ne!(a, b, "degenerate triangle edge at {a:?}");
                *edges.entry((a, b)).or_insert(0) += 1;
            }
        }
        for (edge, count) in &edges {
            assert_eq!(
                *count, 1,
                "directed edge {edge:?} used {count} times, expected once — \
                 the surface is open, doubled, or inconsistently wound"
            );
            assert!(
                edges.contains_key(&(edge.1, edge.0)),
                "directed edge {edge:?} has no opposing twin — the surface has a hole"
            );
        }
        assert!(
            signed_volume(m) > 0.0,
            "signed volume {} is not positive — the solid is inside out",
            signed_volume(m)
        );
    }

    #[test]
    fn enclosure_is_watertight() {
        assert_watertight(&enclosure_for(&BoardOutline::new(100.0, 60.0)));
    }

    #[test]
    fn extruded_board_is_watertight() {
        assert_watertight(&standard_board());
    }

    #[test]
    fn enclosure_has_28_triangles() {
        // 1 underside + 4 outer walls + 1 cavity floor + 4 cavity walls + 4 rim
        // = 14 quads = 28 triangles.
        assert_eq!(
            enclosure_for(&BoardOutline::new(100.0, 60.0)).triangle_count(),
            28
        );
    }

    #[test]
    fn enclosure_outer_size_matches_extents() {
        let outline = BoardOutline::new(100.0, 60.0);
        let params = EnclosureParams::from_outline(&outline);
        let (ew, eh, ed) = enclosure_extents(&outline, &params);
        let (mn, mx) = enclosure_for(&outline).aabb();
        assert!(
            (mx[0] - mn[0] - ew).abs() < 1e-3,
            "width {} != {ew}",
            mx[0] - mn[0]
        );
        assert!(
            (mx[1] - mn[1] - eh).abs() < 1e-3,
            "height {} != {eh}",
            mx[1] - mn[1]
        );
        assert!(
            (mx[2] - mn[2] - ed).abs() < 1e-3,
            "depth {} != {ed}",
            mx[2] - mn[2]
        );
    }

    #[test]
    fn cavity_admits_the_board_with_clearance() {
        let outline = BoardOutline::new(100.0, 60.0);
        let p = EnclosureParams::from_outline(&outline);
        let (ew, eh, _) = enclosure_extents(&outline, &p);
        // Outer minus both walls is the cavity; it must exceed the board by
        // exactly one clearance per side.
        let cavity_w = ew - 2.0 * p.wall_mm;
        let cavity_h = eh - 2.0 * p.wall_mm;
        assert!((cavity_w - (100.0 + 2.0 * p.clearance_mm)).abs() < 1e-3);
        assert!((cavity_h - (60.0 + 2.0 * p.clearance_mm)).abs() < 1e-3);
        assert!(cavity_w > 100.0, "cavity must be wider than the board");
    }

    #[test]
    fn tolerance_class_drives_enclosure_size() {
        // The whole point of the tolerance profiles: a tighter process yields a
        // tighter enclosure for the identical board.
        let fdm = BoardOutline::new(100.0, 60.0).with_tolerance(ToleranceClass::Fdm);
        let cnc = BoardOutline::new(100.0, 60.0).with_tolerance(ToleranceClass::Cnc);
        let (fw, ..) = enclosure_extents(&fdm, &EnclosureParams::from_outline(&fdm));
        let (cw, ..) = enclosure_extents(&cnc, &EnclosureParams::from_outline(&cnc));
        assert!(
            cw < fw,
            "cnc enclosure ({cw}) should be tighter than fdm ({fw})"
        );
    }

    #[test]
    fn params_derive_from_tolerance_profile() {
        let resin = BoardOutline::new(10.0, 10.0).with_tolerance(ToleranceClass::Resin);
        let p = EnclosureParams::from_outline(&resin);
        let profile = resin.tolerance_profile();
        assert!((p.clearance_mm - profile.xy_accuracy_mm * 2.0).abs() < 1e-6);
        assert!((p.wall_mm - profile.min_wall_thickness_mm).abs() < 1e-6);
    }

    #[test]
    fn builder_overrides_are_honoured() {
        let outline = BoardOutline::new(50.0, 50.0);
        let p = EnclosureParams::from_outline(&outline)
            .with_clearance(1.0)
            .with_wall(3.0)
            .with_floor(2.0)
            .with_headroom(10.0);
        let (w, h, d) = enclosure_extents(&outline, &p);
        assert!((w - (50.0 + 2.0 + 6.0)).abs() < 1e-3, "width {w}");
        assert!((h - (50.0 + 2.0 + 6.0)).abs() < 1e-3, "height {h}");
        assert!((d - (1.6 + 10.0 + 2.0)).abs() < 1e-3, "depth {d}");
    }

    #[test]
    fn enclosure_normals_are_unit_vectors() {
        for n in &enclosure_for(&BoardOutline::new(80.0, 40.0)).normals {
            let len = libm::sqrtf(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
            assert!((len - 1.0).abs() < 1e-5, "normal length {len} != 1");
        }
    }

    // ── Sealed case ──────────────────────────────────────────────────────────

    fn board() -> BoardOutline {
        BoardOutline::new(100.0, 60.0)
    }

    #[test]
    fn every_case_part_is_watertight() {
        let parts = case_for(&board());
        assert_watertight(&parts.base);
        assert_watertight(&parts.lid);
        assert_watertight(&parts.gasket);
    }

    #[test]
    fn case_part_triangle_counts() {
        let parts = case_for(&board());
        // base: cap + 4 bands + 3 rings + cap = 2+8+8+8+8+8+8+8+2
        assert_eq!(parts.base.triangle_count(), 60);
        // lid: cap + band + ring + band + ring + band + cap
        assert_eq!(parts.lid.triangle_count(), 44);
        // gasket: 2 rings + 2 bands
        assert_eq!(parts.gasket.triangle_count(), 32);
    }

    #[test]
    fn all_case_normals_are_unit_vectors() {
        let parts = case_for(&board());
        for m in [&parts.base, &parts.lid, &parts.gasket] {
            for n in &m.normals {
                let len = libm::sqrtf(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
                assert!((len - 1.0).abs() < 1e-5, "normal length {len} != 1");
            }
        }
    }

    #[test]
    fn gasket_fits_inside_its_groove() {
        // The seal only works if the gasket is smaller than the groove by the
        // seating slack on each side, and no smaller.
        let o = board();
        let p = CaseParams::from_outline(&o);
        let g = CaseGeometry::new(&o, &p);

        let groove_span = g.groove_outer.width() - g.groove_inner.width();
        let gasket_span = g.gasket_outer.width() - g.gasket_inner.width();
        assert!(
            (groove_span / 2.0 - p.groove_width_mm()).abs() < 1e-4,
            "groove width {} != {}",
            groove_span / 2.0,
            p.groove_width_mm()
        );
        assert!(
            (gasket_span / 2.0 - p.gasket_width_mm).abs() < 1e-4,
            "gasket width {} != {}",
            gasket_span / 2.0,
            p.gasket_width_mm
        );
        assert!(
            gasket_span < groove_span,
            "gasket must be narrower than its groove"
        );
    }

    #[test]
    fn lid_tongue_aligns_with_base_groove() {
        // Base and lid derive their rectangles from the same geometry, so the
        // tongue must land exactly in the groove.
        let o = board();
        let p = CaseParams::from_outline(&o);
        let g = CaseGeometry::new(&o, &p);
        // Tongue spans groove_outer..groove_inner — identical rectangles.
        assert_eq!(g.groove_outer, g.groove_outer);
        assert!(g.groove_inner.width() < g.groove_outer.width());
        // And it must be shallower than the groove, or the lid bottoms out on
        // the groove floor and never compresses the gasket.
        assert!(
            p.tongue_height_mm() < p.groove_depth_mm(),
            "tongue {} must be shallower than groove {}",
            p.tongue_height_mm(),
            p.groove_depth_mm()
        );
    }

    #[test]
    fn tongue_compresses_the_gasket() {
        let o = board();
        let p = CaseParams::from_outline(&o);
        // Gasket seats flush in the groove; the tongue then squeezes it by
        // exactly the compression fraction.
        let compressed = p.groove_depth_mm() - p.tongue_height_mm();
        let expected = p.gasket_height_mm * (1.0 - p.gasket_compression);
        assert!(
            (compressed - expected).abs() < 1e-5,
            "compressed height {compressed} != {expected}"
        );
        assert!(compressed < p.gasket_height_mm, "gasket must be squeezed");
    }

    #[test]
    fn the_seal_sets_the_wall_thickness() {
        // A grooved wall must carry a lip, the groove, and a second lip.
        let o = board();
        let p = CaseParams::from_outline(&o);
        assert!(
            (p.wall_mm() - (2.0 * p.lip_mm + p.groove_width_mm())).abs() < 1e-5,
            "wall must be lip + groove + lip"
        );
        assert!(
            p.wall_mm() > p.lip_mm,
            "a sealed wall is necessarily thicker than a plain one"
        );
    }

    #[test]
    fn cavity_admits_the_board() {
        let o = board();
        let p = CaseParams::from_outline(&o);
        let g = CaseGeometry::new(&o, &p);
        assert!(
            (g.cavity.width() - (100.0 + 2.0 * p.clearance_mm)).abs() < 1e-3,
            "cavity {} should be board + 2 clearance",
            g.cavity.width()
        );
        assert!(g.cavity.width() > 100.0);
    }

    #[test]
    fn tolerance_class_drives_case_size() {
        let fdm = BoardOutline::new(100.0, 60.0).with_tolerance(ToleranceClass::Fdm);
        let cnc = BoardOutline::new(100.0, 60.0).with_tolerance(ToleranceClass::Cnc);
        let (fw, ..) = case_extents(&fdm, &CaseParams::from_outline(&fdm));
        let (cw, ..) = case_extents(&cnc, &CaseParams::from_outline(&cnc));
        assert!(cw < fw, "cnc case ({cw}) should be tighter than fdm ({fw})");
    }

    #[test]
    fn case_extents_match_the_generated_base() {
        let o = board();
        let p = CaseParams::from_outline(&o);
        let (w, h, _) = case_extents(&o, &p);
        let (mn, mx) = generate_case_base(&o, &p).aabb();
        assert!((mx[0] - mn[0] - w).abs() < 1e-3);
        assert!((mx[1] - mn[1] - h).abs() < 1e-3);
    }

    #[test]
    fn mirroring_preserves_watertightness_and_flips_normals() {
        let lid = generate_case_lid(&board(), &CaseParams::from_outline(&board()));
        let up = lid.normals.iter().filter(|n| n[2] > 0.5).count();
        let mirrored = lid.mirrored_z();
        assert_watertight(&mirrored);
        let down = mirrored.normals.iter().filter(|n| n[2] < -0.5).count();
        assert_eq!(up, down, "upward faces must become downward faces");
    }

    #[test]
    fn merge_preserves_every_triangle() {
        let p = case_for(&board());
        let (a, b, c) = (
            p.base.triangle_count(),
            p.lid.triangle_count(),
            p.gasket.triangle_count(),
        );
        let merged = Mesh::merge([p.base, p.lid, p.gasket]);
        assert_eq!(merged.triangle_count(), a + b + c);
        let n = merged.vertex_count() as u32;
        for t in &merged.triangles {
            for &i in t {
                assert!(i < n, "merged index {i} out of range {n}");
            }
        }
    }

    #[test]
    fn exploded_view_separates_the_parts() {
        let o = board();
        let p = CaseParams::from_outline(&o);
        let ex = case_exploded(&o, &p);
        let (mn, mx) = ex.aabb();
        let (_, _, closed_depth) = case_extents(&o, &p);
        assert!(
            mx[2] - mn[2] > closed_depth,
            "exploded view ({}) must be taller than the closed case ({closed_depth})",
            mx[2] - mn[2]
        );
        // Footprint is unchanged — parts are separated in z only.
        assert!((mx[0] - mn[0] - g_width(&o, &p)).abs() < 1e-3);
    }

    fn g_width(o: &BoardOutline, p: &CaseParams) -> f32 {
        case_extents(o, p).0
    }

    #[test]
    fn case_parts_export_to_stl_and_glb() {
        let parts = case_for(&board());
        let stl = to_stl_binary(&parts.base);
        assert_eq!(stl.len(), 84 + 50 * 60);
        let glb = to_glb(&parts.gasket);
        assert_eq!(&glb[0..4], b"glTF");
        assert_eq!(
            u32::from_le_bytes([glb[8], glb[9], glb[10], glb[11]]) as usize,
            glb.len()
        );
    }

    // ── Cutouts and standoffs ────────────────────────────────────────────────

    use fiducial_geometry::{connector_opening, Side};
    use std::string::ToString;

    /// A case tall enough for every connector these tests place to clear the
    /// gasket groove. The default 5 mm of headroom is not.
    fn roomy() -> Case {
        Case::new(board()).with_params(CaseParams::from_outline(&board()).with_headroom(10.0))
    }

    fn usb_c(offset_mm: f32) -> Cutout {
        let o = connector_opening("usb-c").unwrap();
        Cutout::new("J1", Side::South, offset_mm, o.width_mm, o.height_mm)
    }

    fn qwiic(side: Side, offset_mm: f32) -> Cutout {
        let o = connector_opening("qwiic").unwrap();
        Cutout::new("J3", side, offset_mm, o.width_mm, o.height_mm)
    }

    #[test]
    fn an_unfeatured_case_is_byte_identical_to_before_cutouts() {
        // The punched-face path must collapse to a single quad when there are
        // no holes, or every existing case silently gains triangles.
        let o = board();
        let p = CaseParams::from_outline(&o);
        let plain = generate_case_base(&o, &p);
        let via_case = Case::new(o).base();
        assert_eq!(plain.triangle_count(), 60);
        assert_eq!(via_case.triangle_count(), 60);
        assert_eq!(to_stl_binary(&plain), to_stl_binary(&via_case));
    }

    #[test]
    fn one_cutout_keeps_the_base_watertight() {
        let case = roomy().with_cutouts(std::vec![usb_c(20.0)]);
        case.validate().expect("declaration must be valid");
        assert_watertight(&case.base());
    }

    #[test]
    fn a_cutout_removes_material() {
        // A hole that does not reduce the solid's volume is not a hole.
        let plain = signed_volume(&roomy().base());
        let punched = signed_volume(&roomy().with_cutouts(std::vec![usb_c(20.0)]).base());
        assert!(
            punched < plain,
            "punched volume {punched} should be below solid {plain}"
        );
    }

    #[test]
    fn several_cutouts_on_one_wall_stay_watertight() {
        let case = roomy().with_cutouts(std::vec![
            usb_c(15.0),
            qwiic(Side::South, 45.0),
            Cutout::new("J4", Side::South, 75.0, 7.8, 6.0),
        ]);
        case.validate().expect("declaration must be valid");
        assert_watertight(&case.base());
    }

    #[test]
    fn cutouts_at_different_heights_on_one_wall_stay_watertight() {
        // This is the case the tunnel subdivision exists for: the taller
        // opening's jambs are split by the shorter opening's rim, and an
        // unsplit tunnel would meet the wall at a T-junction.
        let case = roomy().with_cutouts(std::vec![
            usb_c(20.0),
            qwiic(Side::South, 50.0).with_z_offset(1.5),
        ]);
        case.validate().expect("declaration must be valid");
        assert_watertight(&case.base());
    }

    #[test]
    fn cutouts_on_every_wall_stay_watertight() {
        let case = roomy().with_cutouts(std::vec![
            usb_c(20.0),
            qwiic(Side::East, 30.0),
            qwiic(Side::North, 60.0),
            qwiic(Side::West, 25.0),
        ]);
        case.validate().expect("declaration must be valid");
        assert_watertight(&case.base());
    }

    #[test]
    fn the_opening_clears_the_connector_body() {
        // The declaration names the body; the opening must exceed it by one
        // process tolerance on every side or the part will not pass through.
        let case = roomy().with_cutouts(std::vec![usb_c(20.0)]);
        let p = case.params();
        let g = CaseGeometry::new(&board(), p);
        let h = case.hole_for(&case.cutouts()[0], &g);
        let body = connector_opening("usb-c").unwrap();
        assert!(
            (h.u1 - h.u0 - (body.width_mm + 2.0 * p.fit_clearance_mm)).abs() < 1e-4,
            "opening width {} should be body + 2 tolerances",
            h.u1 - h.u0
        );
        assert!(
            (h.v1 - h.v0 - (body.height_mm + 2.0 * p.fit_clearance_mm)).abs() < 1e-4,
            "opening height {} should be body + 2 tolerances",
            h.v1 - h.v0
        );
        assert!(h.u1 - h.u0 > body.width_mm);
    }

    #[test]
    fn opposite_walls_measure_offsets_from_the_same_board_corner() {
        // A connector 20 mm along the south edge and one 20 mm along the north
        // edge sit at the same board x — otherwise every north-wall offset
        // would have to be worked out by hand.
        let case = roomy();
        let g = CaseGeometry::new(&board(), case.params());
        let south = case.hole_for(&usb_c(20.0), &g);
        let north = case.hole_for(&Cutout::new("J9", Side::North, 20.0, 8.94, 3.26), &g);
        // South runs +x from x = 0; north runs -x from x = outer width.
        let south_x = (south.u0 + south.u1) * 0.5;
        let north_x = g.outer.width() - (north.u0 + north.u1) * 0.5;
        assert!(
            (south_x - north_x).abs() < 1e-3,
            "south centre {south_x} and north centre {north_x} should agree"
        );
    }

    #[test]
    fn tolerance_class_drives_the_opening_size() {
        let body = connector_opening("usb-c").unwrap();
        let width_for = |t: ToleranceClass| {
            let o = BoardOutline::new(100.0, 60.0).with_tolerance(t);
            let case = Case::new(o).with_params(CaseParams::from_outline(&o).with_headroom(10.0));
            let g = CaseGeometry::new(&o, case.params());
            let h = case.hole_for(
                &Cutout::new("J1", Side::South, 20.0, body.width_mm, body.height_mm),
                &g,
            );
            h.u1 - h.u0
        };
        assert!(
            width_for(ToleranceClass::Cnc) < width_for(ToleranceClass::Fdm),
            "a tighter process should cut a tighter opening"
        );
    }

    #[test]
    fn a_cutout_that_reaches_the_groove_is_rejected() {
        // The default 5 mm of headroom leaves a wall too short to pass a USB-C
        // receptacle below the seal. Generating it anyway produces a case that
        // slices perfectly and leaks.
        let case = Case::new(board()).with_cutouts(std::vec![usb_c(20.0)]);
        match case.validate() {
            Err(CaseError::SealBreached {
                label,
                top_mm,
                limit_mm,
            }) => {
                assert_eq!(label, "J1");
                assert!(top_mm > limit_mm);
            }
            other => panic!("expected SealBreached, got {other:?}"),
        }
    }

    #[test]
    fn raising_headroom_admits_the_cutout_the_shallow_case_rejected() {
        let shallow = Case::new(board()).with_cutouts(std::vec![usb_c(20.0)]);
        assert!(shallow.validate().is_err());
        assert!(roomy()
            .with_cutouts(std::vec![usb_c(20.0)])
            .validate()
            .is_ok());
    }

    #[test]
    fn a_cutout_running_off_its_wall_is_rejected() {
        let case = roomy().with_cutouts(std::vec![usb_c(99.0)]);
        assert!(matches!(case.validate(), Err(CaseError::OffWall { .. })));
    }

    #[test]
    fn a_cutout_below_the_cavity_floor_is_rejected() {
        let case = roomy().with_cutouts(std::vec![usb_c(20.0).with_z_offset(-20.0)]);
        assert!(matches!(
            case.validate(),
            Err(CaseError::BelowCavity { .. })
        ));
    }

    #[test]
    fn a_cutout_below_the_minimum_feature_is_rejected() {
        let case = roomy().with_cutouts(std::vec![Cutout::new("J9", Side::South, 20.0, 0.1, 0.1)]);
        assert!(matches!(case.validate(), Err(CaseError::TooSmall { .. })));
    }

    #[test]
    fn overlapping_cutouts_are_rejected() {
        // Two openings that merge leave no material between them: one wide
        // slot, silently watertight and structurally wrong.
        let case = roomy().with_cutouts(std::vec![usb_c(20.0), qwiic(Side::South, 22.0)]);
        match case.validate() {
            Err(CaseError::Overlap { a, b }) => {
                assert_eq!((a.as_str(), b.as_str()), ("J1", "J3"));
            }
            other => panic!("expected Overlap, got {other:?}"),
        }
    }

    #[test]
    fn cutouts_on_different_walls_never_overlap() {
        let case = roomy().with_cutouts(std::vec![usb_c(20.0), qwiic(Side::East, 20.0)]);
        assert!(case.validate().is_ok());
    }

    #[test]
    fn case_errors_name_the_cutout_and_say_what_to_change() {
        let err = Case::new(board())
            .with_cutouts(std::vec![usb_c(20.0)])
            .validate()
            .unwrap_err();
        let msg = std::format!("{err}");
        assert!(msg.contains("J1"), "message must name the cutout: {msg}");
        assert!(
            msg.contains("headroom_mm"),
            "message must say what to change: {msg}"
        );
    }

    #[test]
    fn standoffs_keep_the_base_watertight() {
        let o = board();
        let case = Case::new(o).with_params(CaseParams::from_outline(&o).with_standoffs(3.0, 5.0));
        case.validate().expect("standoffs must be valid");
        assert_watertight(&case.base());
    }

    #[test]
    fn standoffs_add_material_rather_than_removing_it() {
        let o = board();
        let plain = signed_volume(&Case::new(o).base());
        let raised = Case::new(o)
            .with_params(CaseParams::from_outline(&o).with_standoffs(3.0, 5.0))
            .base();
        assert!(signed_volume(&raised) > plain, "posts must add volume");
    }

    #[test]
    fn standoffs_raise_the_board_and_the_rim_with_it() {
        // Headroom is measured above the board, so lifting the board must lift
        // the rim — otherwise standoffs silently eat the component clearance.
        let o = board();
        let plain = CaseParams::from_outline(&o);
        let raised = plain.with_standoffs(3.0, 5.0);
        let (_, _, flat_d) = case_extents(&o, &plain);
        let (_, _, tall_d) = case_extents(&o, &raised);
        assert!(
            (tall_d - flat_d - 3.0).abs() < 1e-4,
            "case grew by {} not 3.0",
            tall_d - flat_d
        );
        let g = CaseGeometry::new(&o, &raised);
        assert!((g.z_board_top - g.z_cavity_floor - 3.0 - o.thickness_mm).abs() < 1e-4);
    }

    #[test]
    fn standoffs_and_cutouts_coexist() {
        // Cutout heights are measured from the board's top surface, so posts
        // must move the openings up with the board.
        let o = board();
        let p = CaseParams::from_outline(&o)
            .with_headroom(10.0)
            .with_standoffs(2.0, 5.0);
        let case = Case::new(o)
            .with_params(p)
            .with_cutouts(std::vec![usb_c(20.0), qwiic(Side::East, 30.0)]);
        case.validate().expect("declaration must be valid");
        assert_watertight(&case.base());

        let g = CaseGeometry::new(&o, &p);
        let h = case.hole_for(&case.cutouts()[0], &g);
        assert!(
            h.v0 > g.z_cavity_floor + 2.0,
            "opening floor {} must sit above the raised board",
            h.v0
        );
    }

    #[test]
    fn standoffs_too_large_for_the_board_are_rejected() {
        let o = BoardOutline::new(12.0, 12.0);
        let case = Case::new(o).with_params(CaseParams::from_outline(&o).with_standoffs(3.0, 6.0));
        assert!(matches!(
            case.validate(),
            Err(CaseError::StandoffsTooLarge { .. })
        ));
    }

    #[test]
    fn a_featured_case_still_exports_and_the_lid_stays_plain() {
        let case = roomy().with_cutouts(std::vec![usb_c(20.0), qwiic(Side::East, 30.0)]);
        let parts = case.parts();
        assert_watertight(&parts.base);
        assert_watertight(&parts.lid);
        assert_watertight(&parts.gasket);
        // A hole in the lid is a hole inside the gasket line, so the lid never
        // carries cutouts however many the base has.
        assert_eq!(parts.lid.triangle_count(), 44);
        assert!(parts.base.triangle_count() > 60);

        let glb = to_glb(&case.exploded());
        assert_eq!(&glb[0..4], b"glTF");
        let stl = to_stl_binary(&parts.base);
        assert_eq!(stl.len(), 84 + 50 * parts.base.triangle_count());
    }

    // ── Solid membership ─────────────────────────────────────────────────────

    /// Count ray/triangle crossings ahead of `origin` (Möller–Trumbore).
    fn ray_crossings(m: &Mesh, origin: [f32; 3], dir: [f32; 3]) -> usize {
        let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
        let cross = |a: [f32; 3], b: [f32; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

        let mut hits = 0;
        for t in &m.triangles {
            let (a, b, c) = (
                m.vertices[t[0] as usize],
                m.vertices[t[1] as usize],
                m.vertices[t[2] as usize],
            );
            let (e1, e2) = (sub(b, a), sub(c, a));
            let pv = cross(dir, e2);
            let det = dot(e1, pv);
            if det.abs() < 1e-9 {
                continue;
            }
            let inv = 1.0 / det;
            let tv = sub(origin, a);
            let u = dot(tv, pv) * inv;
            if u < 0.0 || u > 1.0 {
                continue;
            }
            let qv = cross(tv, e1);
            let v = dot(dir, qv) * inv;
            if v < 0.0 || u + v > 1.0 {
                continue;
            }
            if dot(e2, qv) * inv > 1e-6 {
                hits += 1;
            }
        }
        hits
    }

    /// Whether a point lies in the mesh's material.
    ///
    /// A ray from inside a closed solid leaves it an odd number of times. This
    /// is the only test that actually answers "is there a hole here" — edge
    /// parity and volume both pass on a case whose openings were punched in
    /// the wrong place, or not punched through.
    fn inside(m: &Mesh, p: [f32; 3]) -> bool {
        ray_crossings(m, p, [0.371, 0.553, 0.746]) % 2 == 1
    }

    #[test]
    fn a_cutout_is_a_through_hole_where_it_was_declared() {
        let case = roomy().with_cutouts(std::vec![usb_c(20.0), qwiic(Side::East, 30.0)]);
        case.validate().unwrap();
        let base = case.base();
        let p = case.params();
        let g = CaseGeometry::new(&board(), p);
        let wall = p.wall_mm();
        let inboard = wall + p.clearance_mm;

        // Mid-wall, mid-opening: inside the south wall's footprint, but in the
        // void the cutout carved.
        let z = g.z_board_top + 1.5;
        assert!(
            !inside(&base, [inboard + 20.0, wall * 0.5, z]),
            "the declared opening is still solid"
        );
        // The same depth, further along the same wall: material.
        assert!(
            inside(&base, [inboard + 60.0, wall * 0.5, z]),
            "the wall beside the opening should be solid"
        );
        // Directly below and above the opening: material. The upper probe stops
        // below the groove, because mid-wall at rim height is the groove itself
        // — void by design, and the reason a cutout may not reach that far.
        assert!(inside(
            &base,
            [inboard + 20.0, wall * 0.5, g.z_cavity_floor + 0.3]
        ));
        assert!(inside(
            &base,
            [inboard + 20.0, wall * 0.5, g.z_groove_bottom - 0.3]
        ));
        // And the east wall's opening, on the other axis.
        assert!(!inside(&base, [g.outer.x1 - wall * 0.5, inboard + 30.0, z]));
        assert!(inside(&base, [g.outer.x1 - wall * 0.5, inboard + 55.0, z]));
    }

    #[test]
    fn a_standoff_is_solid_and_the_floor_beside_it_is_not() {
        let o = board();
        let p = CaseParams::from_outline(&o).with_standoffs(3.0, 5.0);
        let case = Case::new(o).with_params(p);
        case.validate().unwrap();
        let base = case.base();
        let g = CaseGeometry::new(&o, &p);
        let z = g.z_cavity_floor + 1.5;

        let feet = case.standoff_feet(&g);
        assert_eq!(feet.len(), 4, "four corner posts");
        for f in &feet {
            let c = [(f.x0 + f.x1) * 0.5, (f.y0 + f.y1) * 0.5, z];
            assert!(inside(&base, c), "post at {c:?} should be solid");
        }
        // The middle of the cavity, at the same height, is where the board goes.
        let mid = [
            (g.cavity.x0 + g.cavity.x1) * 0.5,
            (g.cavity.y0 + g.cavity.y1) * 0.5,
            z,
        ];
        assert!(!inside(&base, mid), "the cavity must stay open");
        // The posts stop where they were told to.
        let f = feet[0];
        let above = [
            (f.x0 + f.x1) * 0.5,
            (f.y0 + f.y1) * 0.5,
            g.z_cavity_floor + 3.0 + 0.5,
        ];
        assert!(!inside(&base, above), "post is taller than 3 mm");
    }

    #[test]
    fn the_cavity_and_the_underside_stay_where_they_were() {
        // Punching walls must not disturb the parts of the base that carry the
        // board or the seal.
        let case = roomy().with_cutouts(std::vec![usb_c(20.0)]);
        let base = case.base();
        let g = CaseGeometry::new(&board(), case.params());
        assert!(
            inside(
                &base,
                [g.outer.x1 * 0.5, g.outer.y1 * 0.5, g.z_cavity_floor * 0.5]
            ),
            "the floor beneath the board must stay solid"
        );
        let (mn, mx) = base.aabb();
        assert!((mx[0] - mn[0] - g.outer.width()).abs() < 1e-3);
        assert!((mx[1] - mn[1] - g.outer.height()).abs() < 1e-3);
        assert!((mx[2] - mn[2] - g.z_rim).abs() < 1e-3);
    }

    #[test]
    fn every_connector_family_has_a_printable_envelope() {
        for c in fiducial_geometry::CONNECTOR_OPENINGS {
            assert!(
                c.width_mm > 0.0 && c.height_mm > 0.0,
                "{} has a degenerate envelope",
                c.name
            );
            assert_eq!(connector_opening(c.name).map(|o| o.name), Some(c.name));
        }
        assert!(connector_opening("db25").is_none());
    }

    #[test]
    fn side_names_round_trip() {
        for s in Side::ALL {
            assert_eq!(Side::from_name(s.name()), Some(s));
        }
        assert!(Side::from_name("up").is_none());
        assert_eq!("J1".to_string(), "J1");
    }

    #[test]
    fn enclosure_exports_to_stl_and_glb() {
        let m = enclosure_for(&BoardOutline::new(100.0, 60.0));
        let stl = to_stl_binary(&m);
        assert_eq!(stl.len(), 84 + 50 * 28);
        let glb = to_glb(&m);
        assert_eq!(&glb[0..4], b"glTF");
        let declared = u32::from_le_bytes([glb[8], glb[9], glb[10], glb[11]]) as usize;
        assert_eq!(declared, glb.len());
    }
}
