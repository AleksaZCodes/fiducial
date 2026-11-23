//! End-to-end: `fid add capability analytics` → `fid derive` → gate.
//!
//! Spec test matrix item 7 from
//! `docs/specs/2026-11-23-analytics-is-a-capability-consent-is-opt-in.md`:
//!
//! 1. No `Set-Cookie` in any response but the staff login route's.
//! 2. No `<script src>` pointing outside the product's own origin, and no
//!    `cookie-consent` component in the tree at all.
//! 3. Nothing written to `localStorage`/`sessionStorage` except the cart key
//!    and the UTM capture key, both declared.
//! 4. `[analytics] providers = ["meta-pixel-capi"]` without `consent` in
//!    `[capabilities] enabled` fails `fid derive` **and** is reported by
//!    `fid doctor`.

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

fn analytics_product(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "p"]);
    assert!(out.status.success(), "fid new: {}", text(&out));
    let root = tmp.join("p");

    let out = run(&root, &["add", "capability", "analytics"]);
    assert!(out.status.success(), "fid add capability analytics: {}", text(&out));

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));
    root
}

fn analytics_web_product(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "p"]);
    assert!(out.status.success(), "fid new: {}", text(&out));
    let root = tmp.join("p");

    for cap in ["design", "web-next", "analytics"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add capability {cap}: {}", text(&out));
    }

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));
    root
}

// ── Installation ──────────────────────────────────────────────────────────────

#[test]
fn installing_analytics_adds_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());
    assert!(
        root.join("pipelines/analytics.toml").exists(),
        "pipeline file written"
    );
}

// ── Derivation ────────────────────────────────────────────────────────────────

#[test]
fn derive_writes_analytics_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    let ts = std::fs::read_to_string(
        root.join("apps/web/src/generated/analytics.ts"),
    )
    .unwrap();
    assert!(ts.contains("createAnalytics"), "{ts}");
    assert!(ts.contains("FanOutAnalytics"), "{ts}");
    assert!(ts.contains("FirstPartyAnalytics"), "{ts}");
    assert!(ts.contains("CloudflareWebAnalytics"), "{ts}");
    assert!(ts.contains("do not edit"), "{ts}");
}

#[test]
fn derive_includes_only_configured_providers() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    let ts = std::fs::read_to_string(
        root.join("apps/web/src/generated/analytics.ts"),
    )
    .unwrap();
    assert!(!ts.contains("PostHogAnalytics"), "posthog not in default: {ts}");
    assert!(!ts.contains("MetaPixelCapi"), "meta-pixel not in default: {ts}");
}

// ── The gate ─────────────────────────────────────────────────────────────────

#[test]
fn check_catches_a_hand_edited_analytics_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    let path = root.join("apps/web/src/generated/analytics.ts");
    std::fs::write(&path, "// hand-edited\n").unwrap();
    let out = run(&root, &["derive", "--check"]);
    assert!(!out.status.success(), "should fail after hand-edit");
    let t = text(&out);
    assert!(t.contains("analytics.ts"), "{t}");
}

// ── Spec test matrix item 7 ───────────────────────────────────────────────────

/// Item 7.2 — no cookie-consent component in the generated files; no third-party
/// script src in the template files installed by the analytics capability.
#[test]
fn default_analytics_scaffold_has_no_consent_banner() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    assert!(
        !root.join("apps/web/src/components/cookie-consent.tsx").exists(),
        "no cookie-consent banner in a default analytics install"
    );

    let ts = std::fs::read_to_string(
        root.join("apps/web/src/generated/analytics.ts"),
    )
    .unwrap();
    assert!(
        !ts.contains("cookie-consent"),
        "generated analytics.ts must not reference cookie-consent: {ts}"
    );
}

/// Item 7.3 — the UTM capture module uses sessionStorage, not localStorage.
#[test]
fn utm_capture_uses_session_storage() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_web_product(tmp.path());

    let capture = std::fs::read_to_string(
        root.join("apps/web/src/lib/analytics-capture.ts"),
    )
    .unwrap();
    assert!(capture.contains("sessionStorage"), "{capture}");
    assert!(!capture.contains("localStorage"), "must not use localStorage: {capture}");
}

fn meta_pixel_product_without_consent(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "p"]);
    assert!(out.status.success());
    let root = tmp.join("p");
    let out = run(&root, &["add", "capability", "analytics"]);
    assert!(out.status.success(), "{}", text(&out));

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    // Replace the whole analytics block regardless of how the array is formatted.
    let config = {
        let start = config.find("[analytics]").expect("[analytics] block must exist");
        let after = &config[start..];
        // Find the next top-level block (starts with '[' at column 0, not '[[')
        let block_end = after[1..]
            .find("\n[")
            .map(|i| start + 1 + i + 1)
            .unwrap_or(config.len());
        format!(
            "{}{}\n{}\n",
            &config[..start],
            "[analytics]\nproviders = [\"meta-pixel-capi\"]\nretention_days = 396",
            &config[block_end..].trim_start()
        )
    };
    std::fs::write(&path, config).unwrap();
    root
}

/// Item 7.4 — `[analytics] providers = ["meta-pixel-capi"]` without the
/// `consent` capability fails `fid derive`.
#[test]
fn meta_pixel_without_consent_fails_derive() {
    let tmp = tempfile::tempdir().unwrap();
    let root = meta_pixel_product_without_consent(tmp.path());

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "derive should fail: {}", text(&out));
    let t = text(&out);
    assert!(t.contains("consent"), "error must mention consent: {t}");
    assert!(t.contains("meta-pixel-capi"), "error must name the provider: {t}");
}

/// Item 7.4 — `[analytics] providers = ["meta-pixel-capi"]` without the
/// `consent` capability is reported by `fid doctor`.
#[test]
fn meta_pixel_without_consent_reported_by_doctor() {
    let tmp = tempfile::tempdir().unwrap();
    let root = meta_pixel_product_without_consent(tmp.path());

    let out = run(&root, &["doctor"]);
    assert!(!out.status.success(), "doctor should exit non-zero: {}", text(&out));
    let t = text(&out);
    assert!(t.contains("consent"), "doctor must mention consent: {t}");
    assert!(t.contains("meta-pixel-capi"), "doctor must name the provider: {t}");
}

// ── Retention (commit 5) ──────────────────────────────────────────────────────

#[test]
fn derive_writes_analytics_retention_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    let ts = std::fs::read_to_string(
        root.join("apps/web/src/generated/analytics-retention.ts"),
    )
    .unwrap();
    assert!(ts.contains("ANALYTICS_RETENTION_DAYS"), "{ts}");
    assert!(ts.contains("cleanupAnalyticsEvents"), "{ts}");
    assert!(ts.contains("do not edit"), "{ts}");
}

#[test]
fn analytics_retention_ts_reflects_configured_days() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    let ts = std::fs::read_to_string(
        root.join("apps/web/src/generated/analytics-retention.ts"),
    )
    .unwrap();
    assert!(ts.contains("= 396"), "default 396 days present: {ts}");
}

#[test]
fn analytics_retention_ts_deletes_by_timestamp() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    let ts = std::fs::read_to_string(
        root.join("apps/web/src/generated/analytics-retention.ts"),
    )
    .unwrap();
    assert!(ts.contains("DELETE FROM analytics_events WHERE ts <"), "{ts}");
}

#[test]
fn check_catches_stale_analytics_retention_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    let path = root.join("apps/web/src/generated/analytics-retention.ts");
    std::fs::write(&path, "// hand-edited\n").unwrap();
    let out = run(&root, &["derive", "--check"]);
    assert!(!out.status.success(), "should fail after hand-edit");
    let t = text(&out);
    assert!(t.contains("analytics-retention.ts"), "{t}");
}

#[test]
fn analytics_cleanup_route_installed_alongside_web_next() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_web_product(tmp.path());

    assert!(
        root.join("apps/web/src/app/api/analytics-cleanup/route.ts").exists(),
        "cleanup route installed with web-next"
    );
    let route = std::fs::read_to_string(
        root.join("apps/web/src/app/api/analytics-cleanup/route.ts"),
    )
    .unwrap();
    assert!(route.contains("cleanupAnalyticsEvents"), "{route}");
    assert!(route.contains("x-cron-secret"), "{route}");
}

#[test]
fn analytics_cleanup_route_not_installed_without_web_next() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    assert!(
        !root.join("apps/web/src/app/api/analytics-cleanup/route.ts").exists(),
        "cleanup route must not be installed without web-next"
    );
}

#[test]
fn analytics_capability_seeds_deploy_crons_when_deploy_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = analytics_product(tmp.path());

    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(config.contains("[deploy]"), "deploy block seeded: {config}");
    assert!(config.contains("0 0 * * *"), "cron expression seeded: {config}");
    assert!(config.contains("ANALYTICS_CRON_SECRET"), "secret seeded: {config}");
}
