//! End-to-end: `for/<capability>/` templates install with their target, in
//! either order, and never without it.
//!
//! The bug this covers shipped. The `design` capability wrote four React
//! `.tsx` stubs into `apps/web/src/components/ui/` of every product — a
//! SvelteKit one included, where nothing can import them — and recorded them
//! in `fiducial.lock` as platform-owned. `fid doctor` then *required* the
//! continued presence of dead files, and the product's only way out was a
//! documented workaround.
//!
//! Unit tests in `capability/manifest.rs` cover the derivation. These cover
//! the property a product actually experiences: the right files, through the
//! real binary, on a real scaffold, whatever order `fid add` was run in.

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

const REACT_PICKER: &str = "apps/web/src/components/locale-picker.tsx";
const SVELTE_PICKER: &str = "apps/web/src/lib/components/LocalePicker.svelte";
const REACT_UI_STUB: &str = "apps/web/src/components/ui/button.tsx";

/// A SvelteKit product gets the Svelte picker and none of the React files.
///
/// Installing the app first is the ordinary order.
#[test]
fn a_svelte_product_gets_the_svelte_picker_and_no_react() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");

    let out = run(&root, &["add", "app", "svelte"]);
    assert!(out.status.success(), "add app svelte: {}", text(&out));
    let out = run(&root, &["add", "i18n"]);
    assert!(out.status.success(), "add i18n: {}", text(&out));
    let out = run(&root, &["add", "design"]);
    assert!(out.status.success(), "add design: {}", text(&out));

    assert!(
        root.join(SVELTE_PICKER).exists(),
        "the Svelte picker installs"
    );
    assert!(
        !root.join(REACT_PICKER).exists(),
        "a .tsx component in a SvelteKit app is a file nothing can import"
    );
    assert!(
        !root.join(REACT_UI_STUB).exists(),
        "the design capability's React stubs must not reach a Svelte product — \
         this is the bug the whole mechanism exists for"
    );
}

/// The same, for Next.js: React in, Svelte out.
#[test]
fn a_next_product_gets_the_react_picker_and_no_svelte() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");

    assert!(run(&root, &["add", "app", "next"]).status.success());
    assert!(run(&root, &["add", "i18n"]).status.success());
    assert!(run(&root, &["add", "design"]).status.success());

    assert!(
        root.join(REACT_PICKER).exists(),
        "the React picker installs"
    );
    assert!(
        root.join(REACT_UI_STUB).exists(),
        "design's React stubs belong here"
    );
    assert!(
        !root.join(SVELTE_PICKER).exists(),
        "no Svelte component in a Next app"
    );
}

/// **Install order must not decide what a product has.**
///
/// `i18n` before the app is the awkward order: at the moment it installs,
/// there is no web capability to scope its picker to. Without the back-fill
/// pass, `fid add app svelte` afterwards would leave the product with no
/// picker at all — and nothing would report it, because a file that was never
/// written is not a file that is stale.
#[test]
fn installing_the_app_last_still_installs_its_components() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");

    assert!(run(&root, &["add", "i18n"]).status.success());
    assert!(
        !root.join(SVELTE_PICKER).exists(),
        "nothing to scope to yet, so nothing is written yet"
    );

    let out = run(&root, &["add", "app", "svelte"]);
    assert!(out.status.success(), "add app svelte: {}", text(&out));
    assert!(
        root.join(SVELTE_PICKER).exists(),
        "installing web-svelte back-fills the templates i18n was holding for it"
    );
}

/// A conditional template is platform-owned like any other: recorded in the
/// lock, and therefore checked by `fid doctor`.
///
/// If it were written but not recorded, `fid upgrade` would never merge
/// changes into it and editing it would be invisible — the file would look
/// tracked and be orphaned.
#[test]
fn a_conditional_template_is_tracked_and_the_product_is_clean() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "app", "svelte"]).status.success());
    assert!(run(&root, &["add", "i18n"]).status.success());

    let lock = std::fs::read_to_string(root.join("fiducial.lock")).expect("lock");
    assert!(
        lock.contains(SVELTE_PICKER),
        "the picker is recorded in fiducial.lock:\n{lock}"
    );

    let out = run(&root, &["doctor"]);
    assert!(
        out.status.success(),
        "fid doctor after install: {}",
        text(&out)
    );
}

/// A scaffolded SvelteKit app declares a CSP that **does not block its own
/// hydration**, and `fid doctor` accepts it.
///
/// This is a regression test for a live outage, not a style check. The
/// product this was found on hand-wrote `script-src 'self'` into
/// `hooks.server.ts` — the only file `fid doctor` used to read — which blocks
/// the inline bootstrap SvelteKit emits. The page server-rendered perfectly
/// and every interactive component was dead: a countdown frozen at 00:00:00,
/// a language picker that would not open. Build, typecheck and doctor were
/// all green, which is why it reached production.
///
/// Two things have to stay true, and they pull against each other:
/// the CSP must exist (doctor), and it must be declared where SvelteKit can
/// complete it with the hash of the script it actually emitted.
#[test]
fn a_svelte_scaffold_declares_csp_where_sveltekit_can_hash_its_own_bootstrap() {
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(run(tmp.path(), &["new", "p"]).status.success(), "fid new");
    let root = tmp.path().join("p");
    assert!(run(&root, &["add", "app", "svelte"]).status.success());

    let hooks = std::fs::read_to_string(root.join("apps/web/src/hooks.server.ts"))
        .expect("the capability ships the headers hook");
    let svelte_config =
        std::fs::read_to_string(root.join("apps/web/svelte.config.js")).expect("svelte.config.js");

    assert!(
        svelte_config.contains("csp") && svelte_config.contains("hash"),
        "CSP must be declared in svelte.config.js in hash mode, so SvelteKit \
         can add the hash of its own inline bootstrap:\n{svelte_config}"
    );
    assert!(
        !hooks.contains("\"Content-Security-Policy\":"),
        "a CSP set as a header here is intersected with the framework's, so the \
         hash is lost and the app stops hydrating:\n{hooks}"
    );

    let out = run(&root, &["doctor"]);
    assert!(
        out.status.success(),
        "doctor must accept a CSP declared where it works: {}",
        text(&out)
    );
}
