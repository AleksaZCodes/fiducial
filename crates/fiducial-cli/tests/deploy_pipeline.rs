//! End-to-end: `fid add deploy` → `fid derive` → `wrangler.toml`.
//!
//! The property that matters: **a vendor selection in `[adapters]` is what
//! puts a binding in `wrangler.toml`, under the name that vendor's adapter
//! actually reads.** Before this pipeline that was hand-copied, and the
//! `worker-cloudflare` template's own commented example had it wrong —
//! binding `MY_DB` where `D1Database` reads `DB`.

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

/// Append an `[adapters]` block, then fill in the ids those vendors need.
fn product_with_deploy(tmp: &Path, adapters: &str) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    assert!(
        run(&root, &["add", "deploy"]).status.success(),
        "fid add deploy"
    );

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    let config = config
        .replace("REPLACE_WITH_DATABASE_ID", "db-id-1")
        .replace("REPLACE_WITH_BUCKET_NAME", "p-assets")
        .replace("REPLACE_WITH_QUEUE_NAME", "p-jobs");
    std::fs::write(&path, format!("{config}\n[adapters]\n{adapters}\n")).unwrap();
    root
}

fn wrangler(root: &Path) -> String {
    std::fs::read_to_string(root.join("apps/worker/wrangler.toml")).unwrap()
}

// ── Installation ──────────────────────────────────────────────────────────────

#[test]
fn installing_deploy_adds_the_pipeline_and_seeds_the_declaration() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "deploy"]).status.success());

    assert!(root.join("pipelines/deploy.toml").exists());
    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(config.contains("[deploy]"), "{config}");
    assert!(config.contains("compatibility_date"), "{config}");
}

// ── Derivation: a selection is what creates a binding ─────────────────────────

#[test]
fn selecting_d1_puts_a_db_binding_in_wrangler_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(tmp.path(), "deploy = \"cloudflare\"\ndatabase = \"d1\"");
    assert!(run(&root, &["derive"]).status.success());

    let w = wrangler(&root);
    assert!(w.contains("[[d1_databases]]"), "{w}");
    // The binding name is the one D1Database reads — not a name a human chose.
    assert!(w.contains("binding = \"DB\""), "{w}");
    assert!(w.contains("database_id = \"db-id-1\""), "{w}");
}

#[test]
fn selecting_r2_and_queues_puts_their_bindings_in_too() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(
        tmp.path(),
        "deploy = \"cloudflare\"\nstorage = \"r2\"\nqueue = \"cloudflare-queues\"",
    );
    assert!(run(&root, &["derive"]).status.success());

    let w = wrangler(&root);
    assert!(w.contains("binding = \"BUCKET\""), "{w}");
    assert!(w.contains("bucket_name = \"p-assets\""), "{w}");
    assert!(w.contains("[[queues.producers]]"), "{w}");
    assert!(w.contains("binding = \"QUEUE\""), "{w}");
}

/// The other half of the property: **not** selecting a vendor leaves its
/// binding out. A `wrangler.toml` that accretes blocks nobody selected is the
/// hand-copied file again, just generated.
#[test]
fn an_unselected_vendor_contributes_no_binding() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(tmp.path(), "deploy = \"cloudflare\"\ndatabase = \"d1\"");
    assert!(run(&root, &["derive"]).status.success());

    let w = wrangler(&root);
    assert!(w.contains("[[d1_databases]]"), "{w}");
    assert!(!w.contains("r2_buckets"), "storage was never selected: {w}");
    assert!(!w.contains("queues.producers"), "no queue selected: {w}");
}

/// Deselecting a vendor removes its binding on the next derive — the whole
/// point of deriving rather than hand-editing.
#[test]
fn deselecting_a_vendor_removes_its_binding() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(
        tmp.path(),
        "deploy = \"cloudflare\"\ndatabase = \"d1\"\nstorage = \"r2\"",
    );
    assert!(run(&root, &["derive"]).status.success());
    assert!(wrangler(&root).contains("r2_buckets"));

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        config.replace("storage = \"r2\"", "storage = \"none\""),
    )
    .unwrap();

    assert!(run(&root, &["derive"]).status.success());
    assert!(
        !wrangler(&root).contains("r2_buckets"),
        "{}",
        wrangler(&root)
    );
}

// ── Secrets are named, never written ─────────────────────────────────────────

#[test]
fn secrets_are_named_as_commands_to_run_never_as_values() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(
        tmp.path(),
        "deploy = \"cloudflare\"\nbotProtection = \"turnstile\"\nauth = \"supabase\"",
    );
    assert!(run(&root, &["derive"]).status.success());

    let w = wrangler(&root);
    assert!(
        w.contains("wrangler secret put TURNSTILE_SECRET_KEY"),
        "{w}"
    );
    assert!(w.contains("wrangler secret put SUPABASE_ANON_KEY"), "{w}");
    // Named in a comment, and only in a comment.
    for line in w.lines() {
        if line.contains("TURNSTILE_SECRET_KEY") || line.contains("SUPABASE_ANON_KEY") {
            assert!(
                line.trim_start().starts_with('#'),
                "a secret must never appear as an assignment: {line}"
            );
        }
    }
}

// ── Validation ───────────────────────────────────────────────────────────────

#[test]
fn a_placeholder_id_fails_for_a_vendor_the_product_selected() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "deploy"]).status.success());

    // Select d1 but leave the seeded REPLACE_ placeholder in place.
    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{config}\n[adapters]\ndeploy = \"cloudflare\"\ndatabase = \"d1\"\n"),
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("d1_database_id"), "{}", text(&out));
    assert!(text(&out).contains("wrangler d1 create"), "{}", text(&out));
}

/// The same placeholder is fine when its vendor was never selected — demanding
/// an id for a vendor the product does not use would make the seed unusable.
#[test]
fn a_placeholder_id_is_ignored_for_a_vendor_the_product_did_not_select() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(run(tmp.path(), &["new", "p"]).status.success());
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "deploy"]).status.success());

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{config}\n[adapters]\ndeploy = \"cloudflare\"\n"),
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn a_missing_compatibility_date_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(tmp.path(), "deploy = \"cloudflare\"");

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        config.replace("compatibility_date = \"2025-01-01\"", ""),
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("compatibility_date"), "{}", text(&out));
}

/// Selecting a deploy vendor this executor cannot write is refused by name,
/// rather than silently emitting a Cloudflare file for a Vercel product.
#[test]
fn a_non_cloudflare_deploy_vendor_is_refused_by_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(tmp.path(), "deploy = \"vercel\"");

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("vercel"), "{}", text(&out));
}

// ── Resend secrets ────────────────────────────────────────────────────────────

#[test]
fn selecting_email_resend_emits_resend_api_key_secret() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(tmp.path(), "deploy = \"cloudflare\"\nemail = \"resend\"");
    assert!(run(&root, &["derive"]).status.success());

    let w = wrangler(&root);
    assert!(w.contains("wrangler secret put RESEND_API_KEY"), "{w}");
    for line in w.lines() {
        if line.contains("RESEND_API_KEY") {
            assert!(
                line.trim_start().starts_with('#'),
                "a secret must never appear as an assignment: {line}"
            );
        }
    }
}

#[test]
fn selecting_newsletter_resend_emits_both_resend_secrets() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(
        tmp.path(),
        "deploy = \"cloudflare\"\nnewsletter = \"resend\"",
    );
    assert!(run(&root, &["derive"]).status.success());

    let w = wrangler(&root);
    assert!(w.contains("wrangler secret put RESEND_API_KEY"), "{w}");
    assert!(w.contains("wrangler secret put RESEND_AUDIENCE_ID"), "{w}");
}

#[test]
fn selecting_both_resend_email_and_newsletter_emits_resend_api_key_exactly_once() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(
        tmp.path(),
        "deploy = \"cloudflare\"\nemail = \"resend\"\nnewsletter = \"resend\"",
    );
    assert!(run(&root, &["derive"]).status.success());

    let w = wrangler(&root);
    // RESEND_API_KEY must appear in exactly one `wrangler secret put` comment.
    let count = w
        .lines()
        .filter(|l| l.contains("wrangler secret put RESEND_API_KEY"))
        .count();
    assert_eq!(count, 1, "RESEND_API_KEY must not be duplicated:\n{w}");
    assert!(w.contains("wrangler secret put RESEND_AUDIENCE_ID"), "{w}");
}

// ── Gate ─────────────────────────────────────────────────────────────────────

#[test]
fn check_catches_a_hand_edited_wrangler_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(tmp.path(), "deploy = \"cloudflare\"\ndatabase = \"d1\"");
    assert!(run(&root, &["derive"]).status.success());

    let path = root.join("apps/worker/wrangler.toml");
    let w = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, w.replace("binding = \"DB\"", "binding = \"MY_DB\"")).unwrap();

    let out = run(&root, &["derive", "--check"]);
    assert!(
        !out.status.success(),
        "a hand-edited binding must fail --check"
    );
    assert!(text(&out).contains("wrangler.toml"), "{}", text(&out));
}

#[test]
fn selecting_openrouter_emits_the_openrouter_api_key_secret() {
    let tmp = tempfile::tempdir().unwrap();
    let root = product_with_deploy(tmp.path(), "deploy = \"cloudflare\"\nai = \"openrouter\"");
    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{config}\n[ai]\nmodel = \"anthropic/claude-opus-5\"\n"),
    )
    .unwrap();
    assert!(run(&root, &["derive"]).status.success());

    let w = wrangler(&root);
    assert!(w.contains("wrangler secret put OPENROUTER_API_KEY"), "{w}");
    // The model is a declaration, not a secret: it belongs in the generated
    // factory where `fid derive --check` gates it, never in a
    // `wrangler secret put` line and never as a plaintext var beside one.
    assert!(!w.contains("anthropic/claude-opus-5"), "{w}");
}

// ── The SvelteKit shape ──────────────────────────────────────────────────────
//
// A SvelteKit app deployed through `@sveltejs/adapter-cloudflare` is a Worker,
// but not the shape this executor was written for: `apps/web` **is** the
// Worker, the entrypoint and the assets are build output rather than source,
// and the config lives at the product root because that is where the adapter
// looks. Generating `apps/worker/wrangler.toml` for it describes nothing the
// product runs while the real config stays untracked — which is why one
// product declared `[adapters] deploy = "cloudflare"` and deliberately did not
// install this capability.

/// A product whose `[deploy]` describes the SvelteKit shape, with a KV
/// namespace of its own and a JSONC output.
fn sveltekit_product(tmp: &Path) -> PathBuf {
    assert!(run(tmp, &["new", "p"]).status.success(), "fid new");
    let root = tmp.join("p");
    assert!(
        run(&root, &["add", "deploy"]).status.success(),
        "add deploy"
    );

    // `fid add deploy` has already seeded a `[deploy]` block, so this edits
    // that block rather than appending a second one — two `[deploy]` headers
    // is a duplicate key and the config stops parsing at all.
    let config_path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&config_path).unwrap();
    let config = config.replace(
        "[deploy]\n",
        "[deploy]\nshape = \"sveltekit\"\nname = \"upoznaj-biznis\"\n\
         observability = true\nvars = { SPOTS_DEFAULT = \"14\" }\n",
    );
    let config = config
        .replace("\"2025-01-01\"", "\"2026-08-31\"")
        .replace("REPLACE_WITH_DATABASE_ID", "db-id-1")
        .replace("REPLACE_WITH_BUCKET_NAME", "p-assets")
        .replace("REPLACE_WITH_QUEUE_NAME", "p-jobs");
    let config = format!(
        "{config}\n[adapters]\ndeploy = \"cloudflare\"\n\n\
         [[deploy.kv_namespaces]]\nbinding = \"SPOTS\"\n\
         id = \"a6a236c7ea344ddca04548dd4c90fb44\"\n",
    );
    std::fs::write(&config_path, config).unwrap();

    // The output path is what says which format — the same convention the
    // brand pipeline uses, where the file name says which artifact.
    std::fs::write(
        root.join("pipelines/deploy.toml"),
        "name     = \"deploy\"\nexecutor = \"fid-deploy\"\noutputs  = [\"wrangler.jsonc\"]\n",
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));
    root
}

/// The entrypoint and the assets are the adapter's build output.
///
/// Pointing `main` at `src/index.ts` — the standalone Worker's default — names
/// a file a SvelteKit product does not have, and omitting `assets` entirely
/// makes every static file 404 while the Worker itself looks healthy.
#[test]
fn the_sveltekit_shape_points_at_the_adapters_build_output() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = sveltekit_product(tmp.path());
    let cfg = std::fs::read_to_string(root.join("wrangler.jsonc")).expect("wrangler.jsonc");

    assert!(
        cfg.contains("\".svelte-kit/cloudflare/_worker.js\""),
        "{cfg}"
    );
    assert!(
        cfg.contains("\"directory\": \".svelte-kit/cloudflare\""),
        "{cfg}"
    );
    assert!(cfg.contains("\"binding\": \"ASSETS\""), "{cfg}");
    assert!(!cfg.contains("src/index.ts"), "{cfg}");
    assert!(
        !root.join("apps/worker/wrangler.toml").exists(),
        "a SvelteKit product must not also get a bare-Worker config"
    );
}

/// A KV namespace is declared, not derived, and its id is load-bearing.
///
/// No `[adapters]` contract describes "this product's own KV namespace", so
/// there was nowhere to state it — and one product's live signup counter is
/// written by an external script into a namespace whose id has to survive
/// every migration. Point a deploy at a different id and the data is silently
/// gone rather than missing.
#[test]
fn a_products_own_kv_namespace_is_declared_and_survives_derivation() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = sveltekit_product(tmp.path());
    let cfg = std::fs::read_to_string(root.join("wrangler.jsonc")).unwrap();

    assert!(cfg.contains("\"binding\": \"SPOTS\""), "{cfg}");
    assert!(cfg.contains("a6a236c7ea344ddca04548dd4c90fb44"), "{cfg}");
    // The Worker's name is its identity on the account: a changed name is a
    // second deployment, not a rename.
    assert!(cfg.contains("\"name\": \"upoznaj-biznis\""), "{cfg}");
    assert!(cfg.contains("\"SPOTS_DEFAULT\": \"14\""), "{cfg}");
    assert!(
        cfg.contains("\"observability\": { \"enabled\": true }"),
        "{cfg}"
    );
}

/// The generated JSONC is still JSON once the comments are taken out.
///
/// The comments are deliberate — a generated file a reader does not know is
/// generated is one they hand-edit and lose — but a config `wrangler` cannot
/// parse is worse than no config.
#[test]
fn the_generated_jsonc_parses_once_comments_are_stripped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = sveltekit_product(tmp.path());
    let cfg = std::fs::read_to_string(root.join("wrangler.jsonc")).unwrap();

    let stripped: String = cfg
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let value: serde_json::Value =
        serde_json::from_str(&stripped).unwrap_or_else(|e| panic!("not JSON: {e}\n{stripped}"));

    assert_eq!(value["name"], "upoznaj-biznis");
    assert_eq!(value["kv_namespaces"][0]["binding"], "SPOTS");
    assert_eq!(value["assets"]["binding"], "ASSETS");
}

/// `fid derive --check` gates it like any other artifact.
#[test]
fn a_hand_edited_sveltekit_config_fails_the_gate() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = sveltekit_product(tmp.path());
    let path = root.join("wrangler.jsonc");

    let cfg = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        cfg.replace("a6a236c7ea344ddca04548dd4c90fb44", "some-other-namespace"),
    )
    .unwrap();

    let out = run(&root, &["derive", "--check"]);
    assert!(
        !out.status.success(),
        "editing a KV id by hand must fail the gate: {}",
        text(&out)
    );
}

/// A shape nobody implements is an error, not a silent fall back to the
/// default — a typo would otherwise generate a valid config for the wrong
/// thing, which is the failure this field exists to end.
#[test]
fn an_unknown_shape_is_refused_by_name() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = sveltekit_product(tmp.path());
    let config_path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&config_path).unwrap();
    std::fs::write(
        &config_path,
        config.replace("shape = \"sveltekit\"", "shape = \"svelte-kit\""),
    )
    .unwrap();

    let out = run(&root, &["derive"]);
    assert!(!out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("svelte-kit"), "{}", text(&out));
}
