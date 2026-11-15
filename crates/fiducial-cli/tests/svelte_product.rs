//! A SvelteKit product with the web capabilities: what it must and must not get.
//!
//! The SvelteKit templates had never been compiled: no Tailwind for the
//! stylesheet that imports it, a TypeScript `svelte-check` cannot run, React
//! files from `legal`, an adapter that builds nothing deployable. The network
//! half is `scripts/verify-web-svelte.sh`; this is the half that needs none.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(PathBuf::from(env!("CARGO_BIN_EXE_fid")))
        .args(args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .output()
        .expect("running fid")
}

fn ok(cwd: &Path, args: &[&str]) {
    let out = run(cwd, args);
    assert!(
        out.status.success(),
        "fid {}: {}{}",
        args.join(" "),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_sveltekit_product_gets_svelte_files_and_a_deployable_build() {
    let tmp = tempfile::tempdir().unwrap();
    ok(
        tmp.path(),
        &["new", "s", "--locales", "en,sr", "--default-locale", "en"],
    );
    let root = tmp.path().join("s");
    ok(&root, &["add", "design"]);
    ok(&root, &["add", "app", "svelte"]);
    ok(&root, &["add", "brand"]);
    ok(&root, &["add", "legal"]);
    ok(&root, &["add", "deploy"]);
    ok(&root, &["derive"]);
    ok(&root, &["derive", "--check"]);

    // No React in a SvelteKit app: `legal`'s components are for Next.js only.
    let web = root.join("apps/web");
    assert!(!web.join("src/components/legal-document.tsx").exists());
    assert!(!web.join("src/components/cookie-consent.tsx").exists());

    // One workspace at the root, so the app and the editor install together.
    assert!(root.join("pnpm-workspace.yaml").exists());

    // The app is the Worker, built by the Cloudflare adapter.
    let config = std::fs::read_to_string(root.join("fiducial.toml")).unwrap();
    assert!(config.contains("shape = \"sveltekit\""), "{config}");
    assert!(web.join("wrangler.jsonc").exists());
    let svelte = std::fs::read_to_string(web.join("svelte.config.js")).unwrap();
    assert!(svelte.contains("@sveltejs/adapter-cloudflare"), "{svelte}");

    // What the stylesheet and the checker need is installed.
    let pkg = std::fs::read_to_string(web.join("package.json")).unwrap();
    for dep in [
        "\"tailwindcss\"",
        "\"@tailwindcss/vite\"",
        "\"@fiducial/i18n\"",
    ] {
        assert!(pkg.contains(dep), "{dep}: {pkg}");
    }
    assert!(pkg.contains("svelte-kit sync && svelte-check"), "{pkg}");

    // Only the declared regions' flags, derived from the catalogs.
    let flags = std::fs::read_to_string(web.join("src/generated/flags.ts")).unwrap();
    assert!(
        flags.contains("import { GB, RS } from \"country-flag-icons/string/3x2\""),
        "{flags}"
    );
}
