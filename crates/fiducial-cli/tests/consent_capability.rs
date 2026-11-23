use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fid() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fid"))
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

fn consent_product(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "p"]);
    assert!(out.status.success(), "fid new: {}", text(&out));
    let root = tmp.join("p");

    for cap in ["brand", "i18n", "legal", "consent"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add capability {cap}: {}", text(&out));
    }

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));
    root
}

fn consent_web_product(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "p"]);
    assert!(out.status.success(), "fid new: {}", text(&out));
    let root = tmp.join("p");

    for cap in ["brand", "i18n", "legal", "consent", "design", "web-next"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add capability {cap}: {}", text(&out));
    }

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));
    root
}

#[test]
fn installing_consent_adds_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = consent_product(tmp.path());
    assert!(
        root.join("pipelines/consent.toml").exists(),
        "consent pipeline written"
    );
}

#[test]
fn derive_writes_consent_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = consent_product(tmp.path());

    let ts = std::fs::read_to_string(
        root.join("apps/web/src/generated/consent.ts"),
    )
    .unwrap();
    assert!(ts.contains("CookieCategory"), "{ts}");
    assert!(ts.contains("cookieConsent"), "{ts}");
    assert!(ts.contains("hasConsent"), "{ts}");
    assert!(ts.contains("recordConsent"), "{ts}");
}

#[test]
fn consent_ts_starts_with_nothing_consented() {
    let tmp = tempfile::tempdir().unwrap();
    let root = consent_product(tmp.path());

    let ts = std::fs::read_to_string(
        root.join("apps/web/src/generated/consent.ts"),
    )
    .unwrap();
    assert!(
        ts.contains("return cookieConsent()?.granted.includes(category) ?? false;"),
        "hasConsent must default to false: {ts}"
    );
}

#[test]
fn check_catches_a_hand_edited_consent_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = consent_product(tmp.path());

    let path = root.join("apps/web/src/generated/consent.ts");
    let ts = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, ts.replace("?? false;", "?? true;")).unwrap();

    let out = run(&root, &["derive", "--check"]);
    assert!(!out.status.success(), "should fail after hand-edit");
    assert!(text(&out).contains("consent.ts"), "{}", text(&out));
}

#[test]
fn banner_component_installed_with_web_next() {
    let tmp = tempfile::tempdir().unwrap();
    let root = consent_web_product(tmp.path());

    let banner = std::fs::read_to_string(
        root.join("apps/web/src/components/cookie-consent.tsx"),
    )
    .unwrap();
    assert!(banner.contains("from \"../generated/consent\""), "{banner}");
    assert!(!banner.contains("?? false"), "the banner decides nothing: {banner}");
}

#[test]
fn consent_capability_requires_legal() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success());
    let root = tmp.path().join("p");

    let out = run(&root, &["add", "capability", "consent"]);
    assert!(!out.status.success(), "consent without legal should fail: {}", text(&out));
    let t = text(&out);
    assert!(
        t.contains("legal") || t.contains("requires"),
        "error must mention legal: {t}"
    );
}
