//! End-to-end tests for `fid release`.
//!
//! Proves the "done when" criterion for Phase 16b:
//!   - `fid release status` renders without error
//!   - `fid release check` passes when the matrix matches WIRE_VERSION
//!   - `fid release check` fails when the matrix is out of sync
//!   - `fid release protocol --bump breaking` updates the matrix correctly
//!   - A protocol bump makes old-version artifacts fail the skew check

use std::{fs, path::Path, process::Command};

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

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into()
}

/// Workspace root — where `docs/compat/matrix.toml` lives.
fn workspace_root() -> &'static Path {
    // CARGO_MANIFEST_DIR = crates/fiducial-cli; ../../ = repo root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

// ── status ────────────────────────────────────────────────────────────────────

#[test]
fn status_exits_zero() {
    let root = workspace_root();
    let out = run(root, &["release", "status"]);
    assert!(
        out.status.success(),
        "`fid release status` failed:\n{}",
        stderr(&out)
    );
    let text = stdout(&out);
    assert!(
        text.contains("wire version"),
        "expected 'wire version' in output:\n{text}"
    );
}

// ── check — happy path ────────────────────────────────────────────────────────

#[test]
fn check_passes_with_committed_matrix() {
    // The committed matrix.toml should already agree with the compiled WIRE_VERSION.
    let root = workspace_root();
    let out = run(root, &["release", "check"]);
    assert!(
        out.status.success(),
        "`fid release check` failed — matrix is out of sync:\n{}\n{}",
        stdout(&out),
        stderr(&out)
    );
    let text = stdout(&out);
    assert!(
        text.contains("in sync"),
        "expected 'in sync' in output:\n{text}"
    );
}

// ── check — out-of-sync matrix ────────────────────────────────────────────────

#[test]
fn check_fails_when_matrix_has_wrong_version() {
    // Write a temporary matrix.toml with current=99 into a scratch directory,
    // then run `fid release check` from there.
    let tmp = tempfile::TempDir::new().unwrap();
    let compat_dir = tmp.path().join("docs").join("compat");
    fs::create_dir_all(&compat_dir).unwrap();

    let stale_matrix = r#"
[wire]
current        = 99
min_compatible = 99

[[history]]
version = 99
date    = "2026-09-10"
notes   = "Stale test fixture."
"#;
    fs::write(compat_dir.join("matrix.toml"), stale_matrix).unwrap();

    let out = run(tmp.path(), &["release", "check"]);
    assert!(
        !out.status.success(),
        "`fid release check` should have failed when matrix.current=99 ≠ WIRE_VERSION:\n{}",
        stdout(&out)
    );
    let combined = format!("{}{}", stdout(&out), stderr(&out));
    assert!(
        combined.contains("skew") || combined.contains("99"),
        "expected version-skew error mentioning '99':\n{combined}"
    );
}

// ── protocol bump — breaking ──────────────────────────────────────────────────

#[test]
fn bump_breaking_updates_matrix() {
    let tmp = tempfile::TempDir::new().unwrap();
    let compat_dir = tmp.path().join("docs").join("compat");
    fs::create_dir_all(&compat_dir).unwrap();

    // Start from current wire version.
    let wire_ver = fiducial_protocol::WIRE_VERSION;
    let initial = format!(
        r#"
[wire]
current        = {wire_ver}
min_compatible = {wire_ver}

[[history]]
version = {wire_ver}
date    = "2026-09-10"
notes   = "Initial."
"#
    );
    fs::write(compat_dir.join("matrix.toml"), initial).unwrap();

    let out = run(
        tmp.path(),
        &[
            "release",
            "protocol",
            "--bump",
            "breaking",
            "--note",
            "Test breaking bump",
        ],
    );
    assert!(
        out.status.success(),
        "`fid release protocol --bump breaking` failed:\n{}\n{}",
        stdout(&out),
        stderr(&out)
    );

    // Matrix should now have current = wire_ver + 1.
    let raw = fs::read_to_string(compat_dir.join("matrix.toml")).unwrap();
    let new_ver = wire_ver + 1;
    // toml::to_string_pretty uses "current = N" (no padding), but the hand-written
    // initial file uses aligned spacing.  Check for "= {new_ver}" anywhere in the
    // [wire] section rather than exact spacing.
    assert!(
        raw.contains(&format!("current = {new_ver}"))
            || raw.contains(&format!("current        = {new_ver}")),
        "expected current={new_ver} in updated matrix:\n{raw}"
    );
    assert!(
        raw.contains(&format!("min_compatible = {new_ver}")),
        "expected min_compatible={new_ver} in updated matrix:\n{raw}"
    );
    assert!(
        raw.contains("Test breaking bump"),
        "expected note in history:\n{raw}"
    );

    // The output must remind the author to update WIRE_VERSION.
    let text = stdout(&out);
    assert!(
        text.contains("WIRE_VERSION") || text.contains("ACTION REQUIRED"),
        "expected reminder to update WIRE_VERSION:\n{text}"
    );
}

// ── protocol bump — compatible ────────────────────────────────────────────────

#[test]
fn bump_compatible_keeps_min_compatible() {
    let tmp = tempfile::TempDir::new().unwrap();
    let compat_dir = tmp.path().join("docs").join("compat");
    fs::create_dir_all(&compat_dir).unwrap();

    let wire_ver = fiducial_protocol::WIRE_VERSION;
    let initial = format!(
        r#"
[wire]
current        = {wire_ver}
min_compatible = {wire_ver}

[[history]]
version = {wire_ver}
date    = "2026-09-10"
notes   = "Initial."
"#
    );
    fs::write(compat_dir.join("matrix.toml"), initial).unwrap();

    let out = run(tmp.path(), &["release", "protocol", "--bump", "compatible"]);
    assert!(
        out.status.success(),
        "`fid release protocol --bump compatible` failed:\n{}",
        stderr(&out)
    );

    let raw = fs::read_to_string(compat_dir.join("matrix.toml")).unwrap();
    let new_ver = wire_ver + 1;
    // current incremented (toml::to_string_pretty uses "current = N")
    assert!(
        raw.contains(&format!("current = {new_ver}"))
            || raw.contains(&format!("current        = {new_ver}")),
        "expected current={new_ver}:\n{raw}"
    );
    // min_compatible stays at old value
    assert!(
        raw.contains(&format!("min_compatible = {wire_ver}")),
        "expected min_compatible={wire_ver} (unchanged):\n{raw}"
    );
}

// ── version-skew assertion ────────────────────────────────────────────────────

#[test]
fn old_artifact_rejected_after_breaking_bump() {
    // This test proves the "done when" criterion at the library level:
    // after a breaking bump (local=2, min=2), a remote still on v1 is rejected.
    use fiducial_protocol::assert_compatible;

    let local: u8 = 2;
    let remote: u8 = 1; // old artifact
    let min_compatible: u8 = 2;

    let err = assert_compatible(local, remote, min_compatible)
        .expect_err("old artifact should be rejected");
    assert_eq!(err.local, 2);
    assert_eq!(err.remote, 1);
    assert_eq!(err.min_compatible, 2);
}
