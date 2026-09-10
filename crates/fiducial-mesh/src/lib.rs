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

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
use fiducial_geometry::BoardOutline;

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
        let z_rim = p.floor_mm + outline.thickness_mm + p.headroom_mm;
        Self {
            outer,
            groove_outer: outer.inset(p.lip_mm),
            groove_inner: outer.inset(p.lip_mm + p.groove_width_mm()),
            cavity: outer.inset(wall),
            gasket_outer: outer.inset(p.lip_mm + p.fit_clearance_mm),
            gasket_inner: outer.inset(p.lip_mm + p.groove_width_mm() - p.fit_clearance_mm),
            z_cavity_floor: p.floor_mm,
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
    let mut b = MeshBuilder::new();

    b.cap(g.outer, 0.0, false);
    b.band(g.outer, 0.0, g.z_rim, true);
    b.ring(g.outer, g.groove_outer, g.z_rim, true);
    b.band(g.groove_outer, g.z_groove_bottom, g.z_rim, false);
    b.ring(g.groove_outer, g.groove_inner, g.z_groove_bottom, true);
    b.band(g.groove_inner, g.z_groove_bottom, g.z_rim, true);
    b.ring(g.groove_inner, g.cavity, g.z_rim, true);
    b.band(g.cavity, g.z_cavity_floor, g.z_rim, false);
    b.cap(g.cavity, g.z_cavity_floor, true);

    b.build()
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

/// An exploded view of the assembled case, for rendering.
///
/// Base at the origin, gasket lifted clear of its groove, lid above that with
/// its tongue pointing down — the arrangement that shows how the seal works.
/// The parts are separated rather than interpenetrating, so nothing z-fights.
///
/// This merges three disjoint solids into one mesh: fine to render or slice,
/// but not a closed manifold.
pub fn case_exploded(outline: &BoardOutline, p: &CaseParams) -> Mesh {
    let g = CaseGeometry::new(outline, p);
    let parts = generate_case(outline, p);
    let gap = (p.headroom_mm * 0.6).max(4.0);

    let gasket_z = g.z_groove_bottom + gap;
    // The lid is modelled tongue-up for printing; mirroring turns it over, and
    // the offset then places its tongue tip one gap above the gasket.
    let lid_z = gasket_z + p.gasket_height_mm + gap + p.lid_thickness_mm + p.tongue_height_mm();

    Mesh::merge([
        parts.base,
        parts.gasket.translated([0.0, 0.0, gasket_z]),
        parts.lid.mirrored_z().translated([0.0, 0.0, lid_z]),
    ])
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
    /// An undirected edge, stored with its endpoints in sorted order.
    type EdgeKey = (VertexKey, VertexKey);

    /// Quantise a vertex to 1 µm so shared corners compare equal despite being
    /// duplicated per-face for flat shading.
    fn key(v: [f32; 3]) -> VertexKey {
        let q = |x: f32| (x * 1000.0).round() as i64;
        (q(v[0]), q(v[1]), q(v[2]))
    }

    /// Every undirected edge of a closed manifold is shared by exactly two
    /// triangles. Anything else means a hole or a duplicate face — a mesh a
    /// slicer would reject.
    fn assert_watertight(m: &Mesh) {
        let mut edges: HashMap<EdgeKey, usize> = HashMap::new();
        for tri in &m.triangles {
            for k in 0..3 {
                let a = key(m.vertices[tri[k] as usize]);
                let b = key(m.vertices[tri[(k + 1) % 3] as usize]);
                let e = if a <= b { (a, b) } else { (b, a) };
                *edges.entry(e).or_insert(0) += 1;
            }
        }
        for (edge, count) in &edges {
            assert_eq!(
                *count, 2,
                "edge {edge:?} shared by {count} triangles, expected 2"
            );
        }
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
