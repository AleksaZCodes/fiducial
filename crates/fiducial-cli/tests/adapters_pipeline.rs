//! End-to-end: `fid add adapters` → `fid derive` → factory file.
//!
//! Mirrors brand_pipeline.rs: tests go through the real binary on a real
//! scaffold, verifying the property that matters — the declared vendors reach
//! the generated TypeScript factory.

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

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A scaffolded product with the adapters capability installed and derived once.
fn product_with_adapters(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    assert!(
        run(&root, &["add", "adapters"]).status.success(),
        "fid add adapters"
    );
    assert!(run(&root, &["derive"]).status.success(), "fid derive");
    root
}

// ── Installation ──────────────────────────────────────────────────────────────

#[test]
fn installing_adapters_adds_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("p");
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    assert!(run(&root, &["add", "adapters"]).status.success());

    // The pipeline file should now exist.
    assert!(
        root.join("pipelines/adapters.toml").exists(),
        "pipeline file written"
    );
}

// ── Derivation ────────────────────────────────────────────────────────────────

#[test]
fn derive_writes_the_factory_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());

    let factory = std::fs::read_to_string(root.join("src/adapters.generated.ts")).unwrap();
    assert!(factory.contains("createAdapters"), "{factory}");
    assert!(factory.contains("NoneDatabase"), "{factory}");
    assert!(factory.contains("NoneStorage"), "{factory}");
    assert!(factory.contains("NoneEmail"), "{factory}");
    assert!(factory.contains("NoneDiagnostics"), "{factory}");
    assert!(factory.contains("NoneBotProtection"), "{factory}");
    assert!(factory.contains("NoneQueue"), "{factory}");
    assert!(factory.contains("NoneAuth"), "{factory}");
    assert!(factory.contains("createAuth"), "{factory}");
    assert!(
        factory.contains("do not edit"),
        "generated header present: {factory}"
    );
}

#[test]
fn factory_imports_from_fiducial_adapters() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());

    let factory = std::fs::read_to_string(root.join("src/adapters.generated.ts")).unwrap();
    assert!(
        factory.contains("@fiducial/adapters"),
        "correct import path: {factory}"
    );
}

// ── Gate ─────────────────────────────────────────────────────────────────────

#[test]
fn check_catches_a_hand_edited_factory() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());

    std::fs::write(root.join("src/adapters.generated.ts"), "// hand-edited\n").unwrap();

    let out = run(&root, &["derive", "--check"]);
    assert!(!out.status.success(), "stale factory must fail --check");
    assert!(
        text(&out).contains("adapters.generated.ts"),
        "{}",
        text(&out)
    );
}

#[test]
fn doctor_is_clean_after_installing_adapters() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());

    let out = run(&root, &["doctor"]);
    assert!(out.status.success(), "{}", text(&out));
}

// ── Real vendors: d1, r2 ─────────────────────────────────────────────────────

/// Select a vendor for a contract by editing `fiducial.toml` directly.
///
/// `[adapters]` is written only when at least one vendor is selected
/// (`Adapters::is_empty` skips it), so a fresh scaffold has no section to
/// find-and-replace into — this appends one entry, adding a new `[adapters]`
/// heading only the first time.
fn set_adapter(root: &Path, contract: &str, vendor: &str) {
    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    let entry = format!("{contract} = \"{vendor}\"");
    let updated = if config.contains("[adapters]") {
        config.replacen("[adapters]", &format!("[adapters]\n{entry}"), 1)
    } else {
        format!("{config}\n[adapters]\n{entry}\n")
    };
    std::fs::write(&path, updated).unwrap();
}

#[test]
fn selecting_d1_derives_the_real_class() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());
    set_adapter(&root, "database", "d1");

    assert!(run(&root, &["derive"]).status.success());
    let factory = std::fs::read_to_string(root.join("src/adapters.generated.ts")).unwrap();
    assert!(factory.contains("D1Database"), "{factory}");
    assert!(factory.contains("@fiducial/adapters/database"), "{factory}");
}

#[test]
fn selecting_r2_derives_the_real_class() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());
    set_adapter(&root, "storage", "r2");

    assert!(run(&root, &["derive"]).status.success());
    let factory = std::fs::read_to_string(root.join("src/adapters.generated.ts")).unwrap();
    assert!(factory.contains("R2Storage"), "{factory}");
    assert!(factory.contains("@fiducial/adapters/storage"), "{factory}");
}

#[test]
fn deriving_with_both_d1_and_r2_selected_writes_both_classes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());
    set_adapter(&root, "database", "d1");
    set_adapter(&root, "storage", "r2");

    assert!(run(&root, &["derive"]).status.success());
    let factory = std::fs::read_to_string(root.join("src/adapters.generated.ts")).unwrap();
    assert!(factory.contains("D1Database"), "{factory}");
    assert!(factory.contains("R2Storage"), "{factory}");
    // Contracts nobody selected still fall back to their no-op.
    assert!(factory.contains("NoneEmail"), "{factory}");
    assert!(factory.contains("NoneDiagnostics"), "{factory}");
}

#[test]
fn selecting_supabase_auth_derives_the_real_class() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());
    set_adapter(&root, "auth", "supabase");

    assert!(run(&root, &["derive"]).status.success());
    let factory = std::fs::read_to_string(root.join("src/adapters.generated.ts")).unwrap();
    assert!(factory.contains("SupabaseAuth"), "{factory}");
    assert!(factory.contains("@fiducial/adapters/auth"), "{factory}");
    // createAuth is a separate, request-scoped factory — auth never joins
    // AdapterSet/createAdapters.
    assert!(
        factory.contains("export function createAuth(env: any, store: AuthKeyValueStore)"),
        "{factory}"
    );
}

#[test]
fn selecting_turnstile_derives_the_real_class() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());
    set_adapter(&root, "botProtection", "turnstile");

    assert!(run(&root, &["derive"]).status.success());
    let factory = std::fs::read_to_string(root.join("src/adapters.generated.ts")).unwrap();
    assert!(factory.contains("Turnstile"), "{factory}");
    assert!(
        factory.contains("@fiducial/adapters/bot-protection"),
        "{factory}"
    );
}

#[test]
fn selecting_cloudflare_queues_derives_the_real_class() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());
    set_adapter(&root, "queue", "cloudflare-queues");

    assert!(run(&root, &["derive"]).status.success());
    let factory = std::fs::read_to_string(root.join("src/adapters.generated.ts")).unwrap();
    assert!(factory.contains("CloudflareQueue"), "{factory}");
    assert!(factory.contains("@fiducial/adapters/queue"), "{factory}");
}

#[test]
fn doctor_still_rejects_an_unimplemented_candidate() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_adapters(tmp.path());
    set_adapter(&root, "database", "supabase");

    let out = run(&root, &["doctor"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(
        text(&out).contains("nothing implements yet"),
        "{}",
        text(&out)
    );
}
