//! End-to-end test for the enclosure derivation chain.
//!
//! Scaffolds a real project, installs the `eda` capability, and drives
//! `fid derive` through the `fid-mesh` executor — the path a user actually
//! takes. Asserts that the generated case parts are structurally valid, that
//! the geometry tracks the declared outline, that the parts fit each other,
//! and that `--check` fails when an artifact goes stale.

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

    assert_eq!(read_stl(&root, "case-base.stl").0, 60, "base triangles");
    assert_eq!(read_stl(&root, "case-lid.stl").0, 44, "lid triangles");
    assert_eq!(read_stl(&root, "gasket.stl").0, 32, "gasket triangles");

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
    edit_board(
        &root,
        "\"tolerance\": \"fdm\"",
        "\"tolerance\": \"fdm\",\n    \"enclosure\": { \"headroom_mm\": 15.0 }",
    );
    assert!(run(&root, &["derive"]).status.success());
    let after = read_stl(&root, "case-base.stl").1;

    assert!(
        (after[2] - before[2] - 10.0).abs() < 1e-3,
        "depth should grow 10mm (5 -> 15 headroom): {} -> {}",
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
        "\"tolerance\": \"fdm\"",
        "\"tolerance\": \"fdm\",\n    \"enclosure\": { \"gasket_compression\": 1.0 }",
    );

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "compression of 1.0 must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("gasket_compression"),
        "error should name the offending field, got: {stderr}"
    );
}
