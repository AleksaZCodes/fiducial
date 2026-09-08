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
}
