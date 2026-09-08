//! End-to-end test for the enclosure derivation chain.
//!
//! Scaffolds a real project, installs the `eda` capability, and drives
//! `fid derive` through the `fid-mesh` executor — the path a user actually
//! takes. Asserts that the generated STL and GLB are structurally valid, that
//! the geometry tracks the declared outline, and that `--check` fails when an
//! artifact goes stale.

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

fn set_outline_field(root: &Path, from: &str, to: &str) {
    let path = root.join("board/board.interface.json");
    let json = std::fs::read_to_string(&path).unwrap();
    assert!(json.contains(from), "expected {from:?} in seed board JSON");
    std::fs::write(&path, json.replace(from, to)).unwrap();
}

#[test]
fn derive_generates_valid_stl_and_glb() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive failed: {out:?}");

    let stl = std::fs::read(root.join("enclosure/board.stl")).expect("board.stl must exist");
    assert!(stl.starts_with(b"fiducial-mesh STL"));
    let (tris, _) = stl_stats(&stl);
    assert_eq!(tris, 28, "enclosure tray is 14 quads");

    let glb = std::fs::read(root.join("enclosure/board.glb")).expect("board.glb must exist");
    assert_eq!(&glb[0..4], b"glTF");
    assert_eq!(u32::from_le_bytes(glb[4..8].try_into().unwrap()), 2);
    assert_eq!(
        u32::from_le_bytes(glb[8..12].try_into().unwrap()) as usize,
        glb.len(),
        "GLB declared length must match the file"
    );
}

#[test]
fn geometry_tracks_the_declared_outline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    assert!(run(&root, &["derive"]).status.success());
    let before = stl_stats(&std::fs::read(root.join("enclosure/board.stl")).unwrap()).1;

    // Widen the board by 20 mm; the enclosure must widen by exactly the same.
    set_outline_field(&root, "\"width_mm\": 100.0", "\"width_mm\": 120.0");
    assert!(run(&root, &["derive"]).status.success());
    let after = stl_stats(&std::fs::read(root.join("enclosure/board.stl")).unwrap()).1;

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
fn tolerance_class_changes_generated_geometry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = scaffold(tmp.path());

    assert!(run(&root, &["derive"]).status.success());
    let fdm = stl_stats(&std::fs::read(root.join("enclosure/board.stl")).unwrap()).1;

    set_outline_field(&root, "\"tolerance\": \"fdm\"", "\"tolerance\": \"resin\"");
    assert!(run(&root, &["derive"]).status.success());
    let resin = stl_stats(&std::fs::read(root.join("enclosure/board.stl")).unwrap()).1;

    assert!(
        resin[0] < fdm[0],
        "resin ({}) should yield a tighter enclosure than fdm ({})",
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
    let stl = root.join("enclosure/board.stl");
    let mut bytes = std::fs::read(&stl).unwrap();
    bytes.extend_from_slice(b"tampered");
    std::fs::write(&stl, bytes).unwrap();

    assert!(
        !run(&root, &["derive", "--check"]).status.success(),
        "--check must fail on a tampered artifact"
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
