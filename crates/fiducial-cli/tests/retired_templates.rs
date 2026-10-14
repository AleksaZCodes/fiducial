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

/// A product-owned file the product deleted stays deleted.
///
/// fon removed the scaffold's `app/layout.tsx` and `app/page.tsx`: its pages
/// live in route groups with their own root layouts. The first upgrade whose
/// templates changed restored both, putting a third root layout into a
/// working app. A deletion is the product's answer for a file it owns.
#[test]
fn a_deleted_product_file_is_not_restored_by_upgrade() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "design"]).status.success());
    assert!(run(&root, &["add", "app", "next"]).status.success());

    let page = "apps/web/src/app/page.tsx";
    std::fs::remove_file(root.join(page)).unwrap();
    // Make the template read as changed upstream since it was installed.
    let lock_path = root.join("fiducial.lock");
    let mut lock: toml::Table = std::fs::read_to_string(&lock_path)
        .unwrap()
        .parse()
        .unwrap();
    lock["templates"][page]["base_content"] = toml::Value::String("// an older page\n".into());
    std::fs::write(&lock_path, toml::to_string(&lock).unwrap()).unwrap();

    let out = run(&root, &["upgrade"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !root.join(page).exists(),
        "the product deleted it; upgrade keeps it deleted"
    );
    // And a second upgrade neither restores it nor re-adds it as "new".
    let out = run(&root, &["upgrade"]);
    assert!(out.status.success());
    assert!(
        !root.join(page).exists(),
        "still deleted after another upgrade"
    );
}
