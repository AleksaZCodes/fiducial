//! A template the platform retires leaves nothing behind.
//!
//! 0.9.5 retired `apps/web/src/app/board/page.tsx`. `fid upgrade` removed the
//! file and left `board/` — an empty directory that Next still reads as a route
//! segment, and that a person then has to notice and delete by hand.

use sha2::{Digest, Sha256};
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

fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(fid())
        .args(args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .output()
        .expect("running fid")
}

/// Plant a retired template as an older platform would have installed it.
fn plant(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    let hash = hex::encode(Sha256::digest(content.as_bytes()));
    let mut lock = std::fs::read_to_string(root.join("fiducial.lock")).unwrap();
    lock.push_str(&format!(
        "\n[templates.\"{rel}\"]\nsource_version = \"0.9.4\"\nhash = \"{hash}\"\nbase_content = \"\"\"\n{content}\"\"\"\n"
    ));
    std::fs::write(root.join("fiducial.lock"), lock).unwrap();
}

#[test]
fn retiring_a_file_removes_the_directories_it_leaves_empty() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");

    plant(
        &root,
        "apps/web/src/app/board/page.tsx",
        "export default function B() {}\n",
    );
    // A sibling the product owns: the walk up must stop at the first
    // directory that still holds something.
    std::fs::write(root.join("apps/web/src/app/layout.tsx"), "// mine\n").unwrap();

    let out = run(&root, &["upgrade"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !root.join("apps/web/src/app/board").exists(),
        "board/ is gone with its page"
    );
    assert!(
        root.join("apps/web/src/app/layout.tsx").exists(),
        "the product's file stays"
    );
}
