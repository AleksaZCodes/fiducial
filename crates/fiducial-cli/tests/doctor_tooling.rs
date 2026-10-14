//! `fid doctor` says how to get a missing tool, not only that it is missing.
//!
//! A hardware product needs KiCad, Java and a Python with pcbnew and ortools.
//! Doctor listed what was absent from PATH and stopped there, so the only
//! description of how to install any of it was the CI workflow's YAML. The
//! hardware capability now ships `hardware/setup.sh`, and doctor names it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fid() -> PathBuf {
    let mut path = std::env::current_exe().expect("test binary path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("fid")
}

fn run(cwd: &Path, args: &[&str], path_env: Option<&str>) -> Output {
    let mut cmd = Command::new(fid());
    cmd.args(args).current_dir(cwd).env("NO_COLOR", "1");
    if let Some(p) = path_env {
        cmd.env("PATH", p);
    }
    cmd.output().expect("running fid")
}

#[test]
fn missing_hardware_tools_point_at_the_setup_script() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"], None).status.success());
    let root = tmp.path().join("p");
    let out = run(&root, &["add", "capability", "hardware"], None);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        root.join("hardware/setup.sh").exists(),
        "the capability ships its setup"
    );

    // An empty PATH: every tool the capability declares is missing.
    let out = run(&root, &["doctor"], Some(""));
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    for tool in ["kicad-cli", "java", "python3"] {
        assert!(
            report.contains(&format!("`{tool}`")),
            "{tool} reported missing: {report}"
        );
    }
    assert!(
        report.contains("`hardware/setup.sh` installs what hardware needs"),
        "and doctor says how to get them: {report}"
    );
}
