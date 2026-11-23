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

fn payload_product(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "p"]);
    assert!(out.status.success(), "fid new: {}", text(&out));
    let root = tmp.join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    let config = config
        .replace("REPLACE_WITH_DATABASE_ID", "db-id-1")
        .replace("REPLACE_WITH_BUCKET_NAME", "p-assets")
        .replace("REPLACE_WITH_QUEUE_NAME", "p-jobs");
    std::fs::write(&path, config).unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));
    root
}

#[test]
fn installing_payload_adds_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    assert!(
        root.join("pipelines/payload.toml").exists(),
        "payload pipeline written"
    );
    assert!(
        root.join("pipelines/payload-codegen.toml").exists(),
        "payload-codegen pipeline written"
    );
}

#[test]
fn admin_page_imports_importmap_from_generated_path() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let page = std::fs::read_to_string(
        root.join("apps/web/src/app/(payload)/admin/[[...segments]]/page.tsx"),
    ).unwrap();
    assert!(
        page.contains("\"../importMap.js\""),
        "admin page must import the generated importMap.js, not the placeholder: {page}"
    );
    assert!(
        !page.contains("\"../../importMap"),
        "admin page must not import from the old placeholder path: {page}"
    );
}

#[test]
fn payload_codegen_outputs_not_applicable_without_node_modules() {
    let tmp = tempfile::tempdir().unwrap();
    let root = payload_product(tmp.path());

    let out = run(&root, &["derive", "--check", "--pipeline", "payload-codegen"]);
    assert!(
        out.status.success(),
        "derive --check --pipeline payload-codegen must pass without node_modules: {}",
        text(&out)
    );
}

#[test]
fn tsconfig_has_erasable_syntax_only() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let tsconfig = std::fs::read_to_string(root.join("apps/web/tsconfig.json")).unwrap();
    assert!(
        tsconfig.contains("erasableSyntaxOnly"),
        "tsconfig.json must include erasableSyntaxOnly: {tsconfig}"
    );
}

#[test]
fn installing_payload_writes_the_capability_files() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    assert!(root.join("apps/web/payload.config.ts").exists());
    assert!(root.join("apps/web/src/payload/access.ts").exists());
    assert!(root.join("apps/web/src/payload/collections/Users.ts").exists());
    assert!(root.join("apps/web/src/payload/collections/Pages.ts").exists());
    assert!(root.join("apps/web/src/payload/collections/Media.ts").exists());
    assert!(root.join("apps/web/src/payload/globals/SiteSettings.ts").exists());
    assert!(root.join("apps/web/src/app/(payload)/layout.tsx").exists());
    assert!(root
        .join("apps/web/src/app/(payload)/admin/[[...segments]]/page.tsx")
        .exists());
    assert!(root
        .join("apps/web/src/app/(payload)/api/[...slug]/route.ts")
        .exists());
    assert!(
        !root.join("apps/web/src/app/(payload)/importMap.ts").exists(),
        "importMap.ts placeholder must not exist — it is generated to importMap.js"
    );
    assert!(root.join("apps/web/scripts/payload-ts-resolve.mjs").exists());
    assert!(root.join("apps/web/scripts/derive-payload-codegen.mjs").exists());
}

#[test]
fn payload_capability_supersedes_next_config() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let next_config = std::fs::read_to_string(root.join("apps/web/next.config.ts")).unwrap();
    assert!(
        next_config.contains("withPayload"),
        "next.config.ts must wrap with withPayload: {next_config}"
    );
}

#[test]
fn payload_capability_supersedes_open_next_config() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let open_next = std::fs::read_to_string(root.join("apps/web/open-next.config.ts")).unwrap();
    assert!(
        open_next.contains("incrementalCache"),
        "open-next.config.ts must enable the R2 incremental cache: {open_next}"
    );
}

#[test]
fn payload_capability_supersedes_tsconfig_with_payload_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let tsconfig = std::fs::read_to_string(root.join("apps/web/tsconfig.json")).unwrap();
    assert!(
        tsconfig.contains("@payload-config"),
        "tsconfig.json must include @payload-config path alias: {tsconfig}"
    );
}

#[test]
fn route_ts_uses_rest_named_exports() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let route = std::fs::read_to_string(
        root.join("apps/web/src/app/(payload)/api/[...slug]/route.ts"),
    )
    .unwrap();
    assert!(route.contains("REST_GET"), "{route}");
    assert!(route.contains("REST_POST"), "{route}");
}

#[test]
fn derive_writes_payload_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = payload_product(tmp.path());

    let ts = std::fs::read_to_string(root.join("apps/web/src/generated/payload.ts")).unwrap();
    assert!(ts.contains("payloadLocales"), "{ts}");
    assert!(ts.contains("defaultLocale"), "{ts}");
    assert!(ts.contains("adminLocale"), "{ts}");
    assert!(ts.contains("entityName"), "{ts}");
    assert!(ts.contains("PayloadRole"), "{ts}");
    assert!(ts.contains("\"owner\""), "{ts}");
    assert!(ts.contains("\"staff\""), "{ts}");
    assert!(ts.contains("\"agency-admin\""), "{ts}");
}

#[test]
fn derive_payload_ts_carries_the_declared_brand_name() {
    let tmp = tempfile::tempdir().unwrap();
    let root = payload_product(tmp.path());

    let ts = std::fs::read_to_string(root.join("apps/web/src/generated/payload.ts")).unwrap();
    assert!(
        ts.contains("Example"),
        "entity name from [brand] must appear: {ts}"
    );
}

#[test]
fn check_catches_a_hand_edited_payload_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = payload_product(tmp.path());

    let path = root.join("apps/web/src/generated/payload.ts");
    std::fs::write(&path, "// hand-edited\n").unwrap();

    let out = run(&root, &["derive", "--check"]);
    assert!(
        !out.status.success(),
        "a hand-edited payload.ts must fail --check"
    );
    assert!(text(&out).contains("payload.ts"), "{}", text(&out));
}

#[test]
fn access_ts_calls_can_from_fiducial_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let access = std::fs::read_to_string(root.join("apps/web/src/payload/access.ts")).unwrap();
    assert!(access.contains("@fiducial/identity"), "{access}");
    assert!(access.contains("can("), "{access}");
}

#[test]
fn installing_payload_writes_the_block_definitions() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    assert!(root.join("apps/web/src/payload/blocks/Hero.ts").exists());
    assert!(root.join("apps/web/src/payload/blocks/Faq.ts").exists());
    assert!(root.join("apps/web/src/payload/blocks/ProductGrid.ts").exists());
    assert!(root.join("apps/web/src/payload/blocks/CustomOrderCta.ts").exists());
}

#[test]
fn pages_collection_references_all_four_blocks() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let pages = std::fs::read_to_string(root.join("apps/web/src/payload/collections/Pages.ts")).unwrap();
    assert!(pages.contains("Hero"), "Pages must import Hero block: {pages}");
    assert!(pages.contains("Faq"), "Pages must import Faq block: {pages}");
    assert!(pages.contains("ProductGrid"), "Pages must import ProductGrid block: {pages}");
    assert!(pages.contains("CustomOrderCta"), "Pages must import CustomOrderCta block: {pages}");
}

#[test]
fn installing_payload_writes_block_renderers() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    assert!(root.join("apps/web/src/components/blocks/HeroBlock.tsx").exists());
    assert!(root.join("apps/web/src/components/blocks/FaqBlock.tsx").exists());
    assert!(root.join("apps/web/src/components/blocks/ProductGridBlock.tsx").exists());
    assert!(root.join("apps/web/src/components/blocks/CustomOrderCtaBlock.tsx").exists());
    assert!(root.join("apps/web/src/components/blocks/BlockRenderer.tsx").exists());
}

#[test]
fn hero_block_renderer_has_explicit_image_sizes() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let hero = std::fs::read_to_string(
        root.join("apps/web/src/components/blocks/HeroBlock.tsx"),
    ).unwrap();
    assert!(hero.contains("sizes="), "HeroBlock must have explicit sizes: {hero}");
}

#[test]
fn custom_order_cta_renderer_has_explicit_image_sizes() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let cta = std::fs::read_to_string(
        root.join("apps/web/src/components/blocks/CustomOrderCtaBlock.tsx"),
    ).unwrap();
    assert!(cta.contains("sizes="), "CustomOrderCtaBlock must have explicit sizes: {cta}");
}

#[test]
fn revalidate_ts_is_written() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    assert!(root.join("apps/web/src/payload/revalidate.ts").exists());
    let rv = std::fs::read_to_string(root.join("apps/web/src/payload/revalidate.ts")).unwrap();
    assert!(rv.contains("revalidatePath"), "revalidate.ts must call revalidatePath: {rv}");
    assert!(rv.contains("revalidateTag"), "revalidate.ts must call revalidateTag: {rv}");
}

#[test]
fn site_settings_global_has_menus_and_banners() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let settings = std::fs::read_to_string(
        root.join("apps/web/src/payload/globals/SiteSettings.ts"),
    ).unwrap();
    assert!(settings.contains("mainNav"), "SiteSettings must have mainNav: {settings}");
    assert!(settings.contains("footerNav"), "SiteSettings must have footerNav: {settings}");
    assert!(settings.contains("banner"), "SiteSettings must have banner: {settings}");
    assert!(settings.contains("seoTitle"), "SiteSettings must have seoTitle: {settings}");
    assert!(settings.contains("companyName"), "SiteSettings must have companyName: {settings}");
    assert!(settings.contains("revalidateSiteSettings"), "SiteSettings must call revalidateSiteSettings: {settings}");
}

#[test]
fn next_config_uses_cloudflare_image_loader() {
    let tmp = tempfile::tempdir().unwrap();
    let out = run(tmp.path(), &["new", "p"]);
    assert!(out.status.success(), "{}", text(&out));
    let root = tmp.path().join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let next_config = std::fs::read_to_string(root.join("apps/web/next.config.ts")).unwrap();
    assert!(
        next_config.contains("cloudflare-image-loader"),
        "next.config.ts must reference the Cloudflare image loader: {next_config}"
    );
    assert!(root.join("apps/web/src/lib/cloudflare-image-loader.ts").exists());
}
