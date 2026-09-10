//! End-to-end test for the enclosure derivation chain.
//!
//! Scaffolds a real project, installs the `eda` capability, and drives
//! `fid derive` through the `fid-mesh` executor — the path a user actually
//! takes. Asserts that the generated case parts are structurally valid, that
//! the geometry tracks the declared outline, that the parts fit each other,
//! that connector mounts become through-holes in the right place, and that
//! `--check` fails when an artifact goes stale.
//!
//! The structural assertions run against the **shipped bytes** rather than the
//! in-memory mesh. A generator that is correct and a writer that is not still
//! produces an unprintable file.

use std::{path::Path, process::Command};

/// Path to the `fid` binary built for this test run.
fn fid_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_fid"))
}

fn run(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(fid_bin())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to invoke fid")
}

/// Scaffold a project with the eda capability installed, returning its root.
fn scaffold(tmp: &Path) -> std::path::PathBuf {
    let out = run(tmp, &["new", "demo"]);
    assert!(out.status.success(), "fid new failed: {out:?}");
    let root = tmp.join("demo");
    let out = run(&root, &["add", "eda"]);
    assert!(out.status.success(), "fid add eda failed: {out:?}");
    root
}

/// Parse a binary STL into (triangle_count, [width, height, depth]).
fn stl_stats(bytes: &[u8]) -> (u32, [f32; 3]) {
    let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap());
    assert_eq!(
        bytes.len(),
        84 + 50 * n as usize,
        "STL length must match its declared triangle count"
    );
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for t in 0..n as usize {
        // Skip the 12-byte face normal, then read three vertices.
        let base = 84 + 50 * t + 12;
        for v in 0..3 {
            for axis in 0..3 {
                let o = base + 12 * v + 4 * axis;
                let c = f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
                lo[axis] = lo[axis].min(c);
                hi[axis] = hi[axis].max(c);
            }
        }
    }
    (n, [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]])
}

/// A vertex position quantised to 1 µm, so faces that duplicate their corners
/// for flat normals still compare equal.
type VertexKey = (i64, i64, i64);
/// A directed edge, from its first endpoint to its second.
type EdgeKey = (VertexKey, VertexKey);

/// Every triangle in a binary STL, as three vertices.
fn stl_triangles(bytes: &[u8]) -> Vec<[[f32; 3]; 3]> {
    let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
    let mut out = Vec::with_capacity(n);
    for t in 0..n {
        let base = 84 + 50 * t + 12; // skip the face normal
        let mut tri = [[0.0f32; 3]; 3];
        for (v, vertex) in tri.iter_mut().enumerate() {
            for (axis, coord) in vertex.iter_mut().enumerate() {
                let o = base + 12 * v + 4 * axis;
                *coord = f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
            }
        }
        out.push(tri);
    }
    out
}

/// Assert the STL is a closed solid whose faces all wind outward.
///
/// Every directed edge must occur exactly once — the definition of a
/// consistently oriented closed manifold, and stronger than the undirected
/// parity a slicer checks, because it also catches a T-junction where a
/// punched wall meets an unpunched neighbour.
fn assert_closed_solid(bytes: &[u8], label: &str) {
    use std::collections::HashMap;
    let tris = stl_triangles(bytes);
    let key = |v: [f32; 3]| -> VertexKey {
        let q = |x: f32| (x * 1000.0).round() as i64;
        (q(v[0]), q(v[1]), q(v[2]))
    };
    let mut edges: HashMap<EdgeKey, usize> = HashMap::new();
    let mut volume = 0.0f64;
    for t in &tris {
        for k in 0..3 {
            *edges.entry((key(t[k]), key(t[(k + 1) % 3]))).or_insert(0) += 1;
        }
        let (a, b, c) = (t[0], t[1], t[2]);
        let cr = [
            (b[1] * c[2] - b[2] * c[1]) as f64,
            (b[2] * c[0] - b[0] * c[2]) as f64,
            (b[0] * c[1] - b[1] * c[0]) as f64,
        ];
        volume += (a[0] as f64 * cr[0] + a[1] as f64 * cr[1] + a[2] as f64 * cr[2]) / 6.0;
    }
    for (e, count) in &edges {
        assert_eq!(*count, 1, "{label}: directed edge {e:?} used {count} times");
        assert!(
            edges.contains_key(&(e.1, e.0)),
            "{label}: edge {e:?} has no opposing twin — the surface is open"
        );
    }
    assert!(
        volume > 0.0,
        "{label}: signed volume {volume} is not positive"
    );
}

/// Count ray/triangle crossings ahead of `origin` (Möller–Trumbore).
fn ray_crossings(tris: &[[[f32; 3]; 3]], origin: [f32; 3], dir: [f32; 3]) -> usize {
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
    for t in tris {
        let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
        let pv = cross(dir, e2);
        let det = dot(e1, pv);
        if det.abs() < 1e-9 {
            continue;
        }
        let inv = 1.0 / det;
        let tv = sub(origin, t[0]);
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

/// Whether a point sits in the solid: a ray leaves a closed solid an odd
/// number of times.
fn in_material(tris: &[[[f32; 3]; 3]], p: [f32; 3]) -> bool {
    ray_crossings(tris, p, [0.371, 0.553, 0.746]) % 2 == 1
}

fn read_stl(root: &Path, name: &str) -> (u32, [f32; 3]) {
    let path = root.join("enclosure").join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{name} must exist: {e}"));
    assert!(bytes.starts_with(b"fiducial-mesh STL"), "{name} STL header");
    stl_stats(&bytes)
}

fn assert_valid_glb(bytes: &[u8], label: &str) {
    assert_eq!(&bytes[0..4], b"glTF", "{label} magic");
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        2,
        "{label} version"
    );
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize,
        bytes.len(),
        "{label} declared length must match the file"
    );
}

/// Board-to-wall clearance on the FDM profile: twice its 0.2 mm XY accuracy.
const FDM_CLEARANCE_MM: f32 = 0.4;

/// Wall thickness of the generated case, recovered from its own footprint.
///
/// Derived rather than hardcoded because the wall is derived: the seal sets it,
/// and declaring a fastener widens it again. A test that hardcodes the number
/// silently starts probing the wrong place the moment the declaration changes.
fn wall_from_footprint(outer_mm: f32, board_mm: f32) -> f32 {
    (outer_mm - board_mm - 2.0 * FDM_CLEARANCE_MM) / 2.0
}

fn edit_board(root: &Path, from: &str, to: &str) {
    let path = root.join("board/board.interface.json");
    let json = std::fs::read_to_string(&path).unwrap();
    assert!(json.contains(from), "expected {from:?} in seed board JSON");
    std::fs::write(&path, json.replace(from, to)).unwrap();
}

#[test]
fn derive_generates_every_case_part() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive failed: {out:?}");

    // The seed board mounts two connectors and stands the board off its floor,
    // so the base carries more faces than the 60 of a plain shell. The lid and
    // gasket never take features, so their counts are fixed.
    assert!(
        read_stl(&root, "case-base.stl").0 > 60,
        "a featured base should carry more than a plain shell"
    );
    // The seed also retains its lid with four screws, so the lid carries bores
    // through its outer lip. The gasket never takes features at all.
    assert!(
        read_stl(&root, "case-lid.stl").0 > 44,
        "a fastened lid should carry more than a plain plate"
    );
    assert_eq!(read_stl(&root, "gasket.stl").0, 32, "gasket triangles");

    for part in ["case-base.stl", "case-lid.stl", "gasket.stl"] {
        let bytes = std::fs::read(root.join("enclosure").join(part)).unwrap();
        assert_closed_solid(&bytes, part);
    }

    let glb = std::fs::read(root.join("enclosure/case.glb")).expect("case.glb must exist");
    assert_valid_glb(&glb, "case.glb");
}

#[test]
fn the_printed_parts_fit_each_other() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    let (_, base) = read_stl(&root, "case-base.stl");
    let (_, lid) = read_stl(&root, "case-lid.stl");
    let (_, gasket) = read_stl(&root, "gasket.stl");

    // Lid must match the base footprint exactly, or it will not close.
    assert!(
        (lid[0] - base[0]).abs() < 1e-3 && (lid[1] - base[1]).abs() < 1e-3,
        "lid {lid:?} must match base footprint {base:?}"
    );
    // The gasket sits inside the rim, so it is strictly smaller in plan.
    assert!(
        gasket[0] < base[0] && gasket[1] < base[1],
        "gasket {gasket:?} must sit inside the base rim {base:?}"
    );
}

#[test]
fn geometry_tracks_the_declared_outline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    assert!(run(&root, &["derive"]).status.success());
    let before = read_stl(&root, "case-base.stl").1;

    // Widen the board by 20 mm; the case must widen by exactly the same.
    edit_board(&root, "\"width_mm\": 100.0", "\"width_mm\": 120.0");
    assert!(run(&root, &["derive"]).status.success());
    let after = read_stl(&root, "case-base.stl").1;

    assert!(
        (after[0] - before[0] - 20.0).abs() < 1e-3,
        "width should grow 20mm: {} -> {}",
        before[0],
        after[0]
    );
    assert!(
        (after[1] - before[1]).abs() < 1e-3,
        "height should be unchanged"
    );
}

#[test]
fn declared_headroom_changes_case_depth() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    assert!(run(&root, &["derive"]).status.success());
    let before = read_stl(&root, "case-base.stl").1;

    // Tall components are declared, not inferred from the process.
    edit_board(&root, "\"headroom_mm\": 10.0", "\"headroom_mm\": 20.0");
    assert!(run(&root, &["derive"]).status.success());
    let after = read_stl(&root, "case-base.stl").1;

    assert!(
        (after[2] - before[2] - 10.0).abs() < 1e-3,
        "depth should grow 10mm (10 -> 20 headroom): {} -> {}",
        before[2],
        after[2]
    );
}

#[test]
fn tolerance_class_changes_generated_geometry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    assert!(run(&root, &["derive"]).status.success());
    let fdm = read_stl(&root, "case-base.stl").1;

    edit_board(&root, "\"tolerance\": \"fdm\"", "\"tolerance\": \"resin\"");
    assert!(run(&root, &["derive"]).status.success());
    let resin = read_stl(&root, "case-base.stl").1;

    assert!(
        resin[0] < fdm[0],
        "resin ({}) should yield a tighter case than fdm ({})",
        resin[0],
        fdm[0]
    );
}

#[test]
fn check_passes_when_fresh_and_fails_when_stale() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    assert!(run(&root, &["derive"]).status.success());
    assert!(
        run(&root, &["derive", "--check"]).status.success(),
        "--check must pass immediately after derive"
    );

    // Hand-editing a generated artifact is exactly what --check exists to catch.
    let stl = root.join("enclosure/case-base.stl");
    let mut bytes = std::fs::read(&stl).unwrap();
    bytes.extend_from_slice(b"tampered");
    std::fs::write(&stl, bytes).unwrap();

    assert!(
        !run(&root, &["derive", "--check"]).status.success(),
        "--check must fail on a tampered artifact"
    );
}

#[test]
fn the_web_glb_copy_is_opt_in() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let pipeline = root.join("pipelines/enclosure.toml");
    let web_glb = root.join("apps/web/public/case.glb");

    // Commented out by default: a product with no web app gets no stray file.
    assert!(run(&root, &["derive"]).status.success());
    assert!(
        !web_glb.exists(),
        "web copy must not be written until opted in"
    );

    // Opting in is uncommenting one line.
    let toml = std::fs::read_to_string(&pipeline).unwrap();
    let opted_in = toml.replace(
        "  # \"apps/web/public/case.glb\",",
        "  \"apps/web/public/case.glb\",",
    );
    assert_ne!(
        toml, opted_in,
        "expected a commented web-copy line to exist"
    );
    std::fs::write(&pipeline, &opted_in).unwrap();

    assert!(run(&root, &["derive"]).status.success());
    let bytes = std::fs::read(&web_glb).expect("web copy must exist once opted in");
    assert_valid_glb(&bytes, "apps/web/public/case.glb");

    // And it is tracked like any other artifact.
    let mut tampered = bytes.clone();
    tampered.extend_from_slice(b"x");
    std::fs::write(&web_glb, tampered).unwrap();
    assert!(
        !run(&root, &["derive", "--check"]).status.success(),
        "--check must cover the opted-in web copy"
    );
}

#[test]
fn derive_reports_a_board_with_no_outline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    // Drop the outline block; the mesh pipeline has nothing to derive from and
    // must say so rather than emitting a degenerate mesh.
    let path = root.join("board/board.interface.json");
    let json = std::fs::read_to_string(&path).unwrap();
    let bi: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut obj = bi.as_object().unwrap().clone();
    obj.remove("outline");
    std::fs::write(&path, serde_json::to_string_pretty(&obj).unwrap()).unwrap();

    let out = run(&root, &["derive"]);
    assert!(
        !out.status.success(),
        "derive should fail without an outline"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("outline"),
        "error should name the missing outline, got: {stderr}"
    );
}

#[test]
fn derive_rejects_an_unknown_part_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    let pipeline = root.join("pipelines/enclosure.toml");

    let toml = std::fs::read_to_string(&pipeline).unwrap();
    std::fs::write(
        &pipeline,
        toml.replace(
            "\"enclosure/gasket.stl\",",
            "\"enclosure/flux-capacitor.stl\",",
        ),
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(
        !out.status.success(),
        "unknown part should fail the pipeline"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("flux-capacitor") && stderr.contains("case-base"),
        "error should name the bad part and list valid ones, got: {stderr}"
    );
}

#[test]
fn invalid_gasket_compression_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    // 1.0 would crush the gasket flat; the declaration should refuse it rather
    // than emit a case that cannot seal.
    edit_board(
        &root,
        "\"headroom_mm\": 10.0",
        "\"headroom_mm\": 10.0,\n      \"gasket_compression\": 1.0",
    );

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "compression of 1.0 must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("gasket_compression"),
        "error should name the offending field, got: {stderr}"
    );
}

// ── Connector cutouts ────────────────────────────────────────────────────────

#[test]
fn a_declared_mount_becomes_a_through_hole_at_the_declared_position() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    let bytes = std::fs::read(root.join("enclosure/case-base.stl")).unwrap();
    assert_closed_solid(&bytes, "case-base.stl");
    let tris = stl_triangles(&bytes);

    // Seed geometry: the board sits 5.8 mm up (1.2 floor + 3.0 standoff + 1.6
    // PCB), J1 is 20 mm along the south edge and J3 30 mm along the east. The
    // wall comes from the footprint, so this holds whatever the seal and the
    // fasteners make it.
    let [w, h, _] = read_stl(&root, "case-base.stl").1;
    let wall = wall_from_footprint(w, 100.0);
    let inboard = wall + FDM_CLEARANCE_MM;
    let z = 7.0f32;

    assert!(
        !in_material(&tris, [inboard + 20.0, wall * 0.5, z]),
        "J1's opening should be void"
    );
    assert!(
        in_material(&tris, [inboard + 60.0, wall * 0.5, z]),
        "the wall beside J1 should be solid"
    );
    assert!(
        !in_material(&tris, [w - wall * 0.5, inboard + 30.0, z]),
        "J3's opening should be void"
    );
    assert!(
        in_material(&tris, [w - wall * 0.5, inboard + 55.0, z]),
        "the wall beside J3 should be solid"
    );
    let _ = h;
}

#[test]
fn removing_a_mount_removes_its_opening() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["derive"]).status.success());
    let before = read_stl(&root, "case-base.stl").0;

    // A connector reached with the lid off declares no mount and gets no hole.
    edit_board(
        &root,
        "],\n      \"mount\": { \"side\": \"east\", \"offset_mm\": 30.0 }",
        "]",
    );
    assert!(run(&root, &["derive"]).status.success());
    let after = read_stl(&root, "case-base.stl").0;

    assert!(
        after < before,
        "dropping a mount should drop faces: {before} -> {after}"
    );
    let bytes = std::fs::read(root.join("enclosure/case-base.stl")).unwrap();
    assert_closed_solid(&bytes, "case-base.stl with one fewer opening");
}

#[test]
fn a_mount_that_reaches_the_gasket_groove_fails_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    // Drop the headroom until the wall is too short to pass a USB-C receptacle
    // below the seal. Generating it anyway yields a case that slices perfectly
    // and leaks, so the pipeline must refuse.
    edit_board(&root, "\"headroom_mm\": 10.0", "\"headroom_mm\": 2.0");
    let out = run(&root, &["derive"]);
    assert!(
        !out.status.success(),
        "a breached seal must fail the pipeline"
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("J1"),
        "error must name the connector: {stderr}"
    );
    assert!(
        stderr.contains("seal"),
        "error must say what breaks: {stderr}"
    );
    assert!(
        stderr.contains("headroom_mm"),
        "error must say what to change: {stderr}"
    );
}

#[test]
fn a_mount_running_off_its_wall_fails_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    edit_board(
        &root,
        "\"side\": \"south\", \"offset_mm\": 20.0",
        "\"side\": \"south\", \"offset_mm\": 99.5",
    );
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "an opening off the wall must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("J1"),
        "error must name the connector: {stderr}"
    );
}

#[test]
fn an_unknown_mount_side_is_rejected_at_the_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    edit_board(&root, "\"side\": \"south\"", "\"side\": \"starboard\"");
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "an unknown side must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("north") && stderr.contains("west"),
        "error should list the valid sides: {stderr}"
    );
}

#[test]
fn a_connector_family_with_no_known_envelope_must_declare_its_size() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    // Guessing a size for an unfamiliar part is how a case ships with a hole
    // the connector does not fit through.
    edit_board(&root, "\"type\": \"usb-c\"", "\"type\": \"db25\"");
    let out = run(&root, &["derive"]);
    assert!(
        !out.status.success(),
        "an unsizable family must be rejected"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("db25"),
        "error must name the type: {stderr}"
    );
    assert!(
        stderr.contains("width_mm"),
        "error must say what to declare: {stderr}"
    );

    // Declaring the envelope makes the same board buildable.
    edit_board(
        &root,
        "\"side\": \"south\", \"offset_mm\": 20.0",
        "\"side\": \"south\", \"offset_mm\": 20.0, \"width_mm\": 12.0, \"height_mm\": 5.0",
    );
    assert!(
        run(&root, &["derive"]).status.success(),
        "an explicit envelope should be accepted"
    );
    let bytes = std::fs::read(root.join("enclosure/case-base.stl")).unwrap();
    assert_closed_solid(&bytes, "case-base.stl with a declared envelope");
}

#[test]
fn the_opening_scales_with_the_declared_body() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    let wide = |root: &Path| {
        let bytes = std::fs::read(root.join("enclosure/case-base.stl")).unwrap();
        let (_, [w, _, _]) = (0u32, stl_stats(&bytes).1);
        let tris = stl_triangles(&bytes);
        // How far along the south wall the opening still reads as void.
        let wall = wall_from_footprint(w, 100.0);
        let inboard = wall + FDM_CLEARANCE_MM;
        (0..400)
            .filter(|i| {
                let x = inboard + 20.0 - 10.0 + *i as f32 * 0.05;
                !in_material(&tris, [x, wall * 0.5, 7.0])
            })
            .count()
    };
    let before = wide(&root);

    edit_board(
        &root,
        "\"side\": \"south\", \"offset_mm\": 20.0",
        "\"side\": \"south\", \"offset_mm\": 20.0, \"width_mm\": 20.0, \"height_mm\": 3.26",
    );
    assert!(run(&root, &["derive"]).status.success());
    let after = wide(&root);

    assert!(
        after > before,
        "a wider declared body should cut a wider opening: {before} -> {after}"
    );
}

// ── Standoffs ────────────────────────────────────────────────────────────────

#[test]
fn declared_standoffs_raise_the_board_and_the_case() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["derive"]).status.success());
    let raised = read_stl(&root, "case-base.stl").1;

    // Headroom is measured above the board, so removing the posts must lower
    // the whole case rather than free up clearance.
    edit_board(&root, "\"standoff_height_mm\": 3.0,", "");
    assert!(run(&root, &["derive"]).status.success());
    let flat = read_stl(&root, "case-base.stl").1;

    assert!(
        (raised[2] - flat[2] - 3.0).abs() < 1e-3,
        "3 mm of standoff should add 3 mm of case: {} vs {}",
        flat[2],
        raised[2]
    );
    let bytes = std::fs::read(root.join("enclosure/case-base.stl")).unwrap();
    assert_closed_solid(&bytes, "case-base.stl without standoffs");
}

#[test]
fn standoffs_are_solid_posts_and_the_cavity_stays_open() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    let bytes = std::fs::read(root.join("enclosure/case-base.stl")).unwrap();
    let tris = stl_triangles(&bytes);
    // Seed posts: 5 mm square, inset one minimum feature (0.8 mm) from the
    // board corners, which start 5.2 mm in from the outer wall.
    let z = 1.2 + 1.5; // mid-post, above the 1.2 mm floor
    let near = 5.2 + 0.8 + 2.5;
    assert!(
        in_material(&tris, [near, near, z]),
        "the corner post should be solid"
    );
    assert!(
        !in_material(&tris, [55.2, 35.2, z]),
        "the middle of the cavity is where the board goes"
    );
}

#[test]
fn standoffs_too_large_for_the_board_fail_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    edit_board(
        &root,
        "\"width_mm\": 100.0,\n    \"height_mm\": 60.0",
        "\"width_mm\": 12.0,\n    \"height_mm\": 12.0",
    );
    edit_board(
        &root,
        "\"standoff_size_mm\": 5.0",
        "\"standoff_size_mm\": 6.0",
    );
    // The board shrank below its connectors, so unmount them — the standoff
    // failure is what this test is about. An explicit null is a mount the
    // declaration deliberately does not have.
    edit_board(
        &root,
        "{ \"side\": \"south\", \"offset_mm\": 20.0 }",
        "null",
    );
    edit_board(&root, "{ \"side\": \"east\", \"offset_mm\": 30.0 }", "null");

    let out = run(&root, &["derive"]);
    assert!(
        !out.status.success(),
        "four posts must not fit a 12 mm board"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("standoff"),
        "error should name standoffs: {stderr}"
    );
}

// ── Fasteners ────────────────────────────────────────────────────────────────

#[test]
fn declared_fasteners_bore_through_the_lip_of_base_and_lid() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["derive"]).status.success());

    for part in ["case-base.stl", "case-lid.stl"] {
        let bytes = std::fs::read(root.join("enclosure").join(part)).unwrap();
        assert_closed_solid(&bytes, part);
    }

    // Seed geometry with a 3 mm screw on the FDM profile: the outer lip is the
    // tessellated bore plus a printable wall either side, and the fastener sits
    // at its midpoint in both axes.
    let [w, h, _] = read_stl(&root, "case-base.stl").1;
    // Fastener centres sit at the midpoint of the outer lip in both axes. The
    // lip is the tessellated bore (3.4 mm clearance, widened by 1/cos(pi/16) so
    // its flats reach that) plus a 1.2 mm wall either side.
    let c = (3.4f32 / (core::f32::consts::PI / 16.0).cos() + 2.4) / 2.0;
    let base = stl_triangles(&std::fs::read(root.join("enclosure/case-base.stl")).unwrap());
    let lid = stl_triangles(&std::fs::read(root.join("enclosure/case-lid.stl")).unwrap());

    for (x, y) in [(c, c), (w - c, c), (w - c, h - c), (c, h - c)] {
        assert!(
            !in_material(&base, [x, y, 7.9]),
            "base bore missing at ({x}, {y})"
        );
        assert!(
            !in_material(&lid, [x, y, 0.85]),
            "lid bore missing at ({x}, {y})"
        );
        // A step outside the bore is still lip material.
        assert!(
            in_material(&base, [x + 2.0, y, 7.9]),
            "the lip beside the bore at ({x}, {y}) should be solid"
        );
    }
}

#[test]
fn removing_the_fastener_declaration_thins_the_case_back_down() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    assert!(run(&root, &["derive"]).status.success());
    let fastened = read_stl(&root, "case-base.stl").1;

    edit_board(&root, ",\n      \"fastener_diameter_mm\": 3.0", "");
    assert!(run(&root, &["derive"]).status.success());
    let plain = read_stl(&root, "case-base.stl").1;

    // The fastener lip has to carry a hole with a printable wall either side,
    // so dropping it narrows the whole case. That cost is the reason fasteners
    // are declared rather than assumed.
    assert!(
        fastened[0] > plain[0] && fastened[1] > plain[1],
        "fastened case {fastened:?} should be larger than plain {plain:?}"
    );
    let bytes = std::fs::read(root.join("enclosure/case-base.stl")).unwrap();
    assert_closed_solid(&bytes, "case-base.stl unfastened");
}

#[test]
fn a_fastener_too_small_to_print_fails_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    edit_board(
        &root,
        "\"fastener_diameter_mm\": 3.0",
        "\"fastener_diameter_mm\": 0.2",
    );
    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "an unprintable fastener must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("fastener_diameter_mm"),
        "error must name the field: {stderr}"
    );
}

#[test]
fn a_non_positive_fastener_is_rejected_at_the_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());
    edit_board(
        &root,
        "\"fastener_diameter_mm\": 3.0",
        "\"fastener_diameter_mm\": -1.0",
    );
    let out = run(&root, &["derive"]);
    assert!(
        !out.status.success(),
        "a negative diameter must be rejected"
    );
}
