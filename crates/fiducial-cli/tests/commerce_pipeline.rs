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

fn commerce_product(tmp: &Path) -> PathBuf {
    let out = run(tmp, &["new", "p"]);
    assert!(out.status.success(), "fid new: {}", text(&out));
    let root = tmp.join("p");

    for cap in ["design", "web-next", "i18n", "brand", "payload", "commerce"] {
        let out = run(&root, &["add", "capability", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let path = root.join("fiducial.toml");
    let config = std::fs::read_to_string(&path).unwrap();
    let config = config
        .replace("REPLACE_WITH_DATABASE_ID", "db-id-1")
        .replace("REPLACE_WITH_BUCKET_NAME", "p-assets")
        .replace("REPLACE_WITH_QUEUE_NAME", "p-jobs");
    // Brand names and international terms that are intentionally identical
    // across locales.  `payload.*` duplicates are pre-existing in the payload
    // capability seed; `commerce.*` ones are brand names (Facebook, Instagram)
    // and the universally recognised word "Email".
    let config = config.replacen(
        "[i18n]\n",
        "[i18n]\nsame = [\
          \"commerce.guestEmail\",\
          \"commerce.leadSourceFacebook\",\
          \"commerce.leadSourceInstagram\",\
          \"payload.adminTitle\",\
          \"payload.emailLabel\"\
        ]\n",
        1,
    );
    std::fs::write(&path, config).unwrap();

    let out = run(&root, &["derive"]);
    assert!(out.status.success(), "fid derive: {}", text(&out));
    root
}

#[test]
fn installing_commerce_adds_the_pipeline() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    assert!(
        root.join("pipelines/commerce.toml").exists(),
        "commerce pipeline written"
    );
}

#[test]
fn installing_commerce_writes_collection_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    for name in &[
        "Products",
        "Variants",
        "Orders",
        "Quotes",
        "Carts",
        "Discounts",
    ] {
        assert!(
            root.join(format!("apps/web/src/payload/collections/{name}.ts"))
                .exists(),
            "{name}.ts missing"
        );
    }
}

#[test]
fn installing_commerce_writes_field_files() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    for name in &["price", "lineItems", "leadSource", "statusTimestamps"] {
        assert!(
            root.join(format!("apps/web/src/payload/fields/{name}.ts"))
                .exists(),
            "fields/{name}.ts missing"
        );
    }
}

#[test]
fn derive_writes_commerce_ts() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let ts =
        std::fs::read_to_string(root.join("apps/web/src/generated/commerce.ts")).unwrap();
    assert!(ts.contains("COMMERCE_CURRENCY"), "{ts}");
    assert!(ts.contains("COMMERCE_CART_ENABLED"), "{ts}");
    assert!(ts.contains("COMMERCE_STATUSES"), "{ts}");
    assert!(ts.contains("COMMERCE_LEAD_SOURCES"), "{ts}");
    assert!(ts.contains("CommerceStatus"), "{ts}");
    assert!(ts.contains("CommerceLeadSource"), "{ts}");
}

#[test]
fn derive_commerce_ts_carries_seed_currency() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let ts =
        std::fs::read_to_string(root.join("apps/web/src/generated/commerce.ts")).unwrap();
    assert!(
        ts.contains("\"RSD\""),
        "seed currency RSD must appear: {ts}"
    );
}

#[test]
fn derive_commerce_ts_carries_seed_statuses() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let ts =
        std::fs::read_to_string(root.join("apps/web/src/generated/commerce.ts")).unwrap();
    for status in &["new", "quoted", "confirmed", "paid", "delivered", "cancelled"] {
        assert!(ts.contains(status), "status {status} must appear: {ts}");
    }
}

#[test]
fn derive_commerce_ts_carries_seed_lead_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let ts =
        std::fs::read_to_string(root.join("apps/web/src/generated/commerce.ts")).unwrap();
    for src in &["direct", "instagram", "facebook", "referral", "other"] {
        assert!(ts.contains(src), "lead source {src} must appear: {ts}");
    }
}

#[test]
fn commerce_supersedes_payload_config() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let cfg = std::fs::read_to_string(root.join("apps/web/payload.config.ts")).unwrap();
    assert!(
        cfg.contains("Products"),
        "payload.config.ts must import Products: {cfg}"
    );
    assert!(
        cfg.contains("Orders"),
        "payload.config.ts must import Orders: {cfg}"
    );
    assert!(
        cfg.contains("Quotes"),
        "payload.config.ts must import Quotes: {cfg}"
    );
}

#[test]
fn commerce_supersedes_product_grid_block() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let block =
        std::fs::read_to_string(root.join("apps/web/src/payload/blocks/ProductGrid.ts"))
            .unwrap();
    assert!(
        block.contains("relationship"),
        "ProductGrid must use a relationship field: {block}"
    );
    assert!(
        !block.contains("productSlugs"),
        "ProductGrid must not use productSlugs array: {block}"
    );
}

#[test]
fn orders_collection_has_status_field() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let orders =
        std::fs::read_to_string(root.join("apps/web/src/payload/collections/Orders.ts"))
            .unwrap();
    assert!(orders.contains("status"), "Orders must have a status field: {orders}");
    assert!(orders.contains("guestName"), "Orders must have guestName: {orders}");
    assert!(orders.contains("guestPhone"), "Orders must have guestPhone: {orders}");
    assert!(orders.contains("mark-paid"), "Orders must have mark-paid endpoint: {orders}");
}

#[test]
fn quotes_collection_has_convert_endpoint() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let quotes =
        std::fs::read_to_string(root.join("apps/web/src/payload/collections/Quotes.ts"))
            .unwrap();
    assert!(
        quotes.contains("convert"),
        "Quotes must have a convert endpoint: {quotes}"
    );
}

#[test]
fn variants_collection_has_dimension_material_color() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let variants =
        std::fs::read_to_string(root.join("apps/web/src/payload/collections/Variants.ts"))
            .unwrap();
    assert!(variants.contains("dimensions"), "Variants must have dimensions: {variants}");
    assert!(variants.contains("material"), "Variants must have material: {variants}");
    assert!(variants.contains("color"), "Variants must have color: {variants}");
}

#[test]
fn discounts_collection_is_hidden() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let discounts =
        std::fs::read_to_string(root.join("apps/web/src/payload/collections/Discounts.ts"))
            .unwrap();
    assert!(
        discounts.contains("hidden: true"),
        "Discounts must be hidden from admin: {discounts}"
    );
}

#[test]
fn fid_derive_check_passes_after_derive() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let out = run(&root, &["derive", "--check"]);
    assert!(
        out.status.success(),
        "fid derive --check must pass after fid derive: {}",
        text(&out)
    );
}

#[test]
fn commerce_installs_dashboard_view() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    assert!(
        root.join("apps/web/src/payload/views/Dashboard.tsx").exists(),
        "Dashboard.tsx must be installed with commerce"
    );
}

#[test]
fn dashboard_is_a_server_component() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let dashboard =
        std::fs::read_to_string(root.join("apps/web/src/payload/views/Dashboard.tsx")).unwrap();
    assert!(dashboard.contains("async"), "Dashboard must be an async server component: {dashboard}");
    assert!(!dashboard.contains("\"use client\""), "Dashboard must not be a client component: {dashboard}");
}

#[test]
fn commerce_installs_metrics_module() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    assert!(
        root.join("apps/web/src/payload/metrics.ts").exists(),
        "metrics.ts must be installed with commerce"
    );
    let metrics =
        std::fs::read_to_string(root.join("apps/web/src/payload/metrics.ts")).unwrap();
    assert!(metrics.contains("getWeeklyOrders"), "{metrics}");
    assert!(metrics.contains("getOpenQuotesCount"), "{metrics}");
    assert!(metrics.contains("getLeadsBySource"), "{metrics}");
    assert!(metrics.contains("getWeeklyFunnel"), "{metrics}");
    assert!(metrics.contains("getAllMetrics"), "{metrics}");
}

#[test]
fn payload_config_registers_dashboard_and_metrics_endpoint() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let cfg = std::fs::read_to_string(root.join("apps/web/payload.config.ts")).unwrap();
    assert!(cfg.contains("Dashboard"), "config must register Dashboard view: {cfg}");
    assert!(cfg.contains("commerce-metrics"), "config must register metrics endpoint: {cfg}");
    assert!(cfg.contains("getAllMetrics"), "config must import getAllMetrics: {cfg}");
}

#[test]
fn commerce_installs_order_route() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    assert!(
        root.join("apps/web/src/app/api/orders/route.ts").exists(),
        "order route missing"
    );
    let route = std::fs::read_to_string(root.join("apps/web/src/app/api/orders/route.ts")).unwrap();
    assert!(route.contains("BotProtection") || route.contains("Turnstile"), "order route must use bot protection contract: {route}");
    assert!(route.contains("order_submitted"), "order route must emit order_submitted event: {route}");
    assert!(route.contains("turnstileToken"), "order route must validate turnstile: {route}");
}

#[test]
fn commerce_installs_quote_route() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    assert!(
        root.join("apps/web/src/app/api/quotes/route.ts").exists(),
        "quote route missing"
    );
    let route = std::fs::read_to_string(root.join("apps/web/src/app/api/quotes/route.ts")).unwrap();
    assert!(route.contains("BotProtection") || route.contains("Turnstile"), "quote route must use bot protection contract: {route}");
    assert!(route.contains("quote_request"), "quote route must emit quote_request event: {route}");
}

#[test]
fn commerce_installs_client_components() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    for name in &["VariantPicker", "OrderForm", "QuoteForm"] {
        let path = root.join(format!("apps/web/src/components/commerce/{name}.tsx"));
        assert!(path.exists(), "{name}.tsx missing");
        let src = std::fs::read_to_string(&path).unwrap();
        assert!(src.contains(r#""use client""#), "{name} must be a client component: {src}");
    }
}

#[test]
fn order_form_reads_utm_from_session_storage() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let form = std::fs::read_to_string(root.join("apps/web/src/components/commerce/OrderForm.tsx")).unwrap();
    assert!(form.contains("sessionStorage"), "OrderForm must read UTM from sessionStorage: {form}");
    assert!(form.contains("whereDidYouFindUs"), "OrderForm must have whereDidYouFindUs field: {form}");
}

#[test]
fn quote_form_has_requirements_field() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let form = std::fs::read_to_string(root.join("apps/web/src/components/commerce/QuoteForm.tsx")).unwrap();
    assert!(form.contains("requirements"), "QuoteForm must have requirements field: {form}");
}

#[test]
fn commerce_adds_adapters_npm_dependency() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let pkg = std::fs::read_to_string(root.join("apps/web/package.json")).unwrap();
    assert!(pkg.contains("@fiducial/adapters"), "package.json must list @fiducial/adapters: {pkg}");
}

#[test]
fn commerce_installs_jobs_file() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let path = root.join("apps/web/src/payload/jobs/index.ts");
    assert!(path.exists(), "jobs/index.ts must be installed with commerce");
    let src = std::fs::read_to_string(&path).unwrap();
    assert!(src.contains("notify-new-order"), "jobs must define notify-new-order task: {src}");
    assert!(src.contains("notify-new-quote"), "jobs must define notify-new-quote task: {src}");
}

#[test]
fn commerce_installs_notify_hook() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let path = root.join("apps/web/src/payload/hooks/notifyOnCreate.ts");
    assert!(path.exists(), "hooks/notifyOnCreate.ts must be installed with commerce");
    let src = std::fs::read_to_string(&path).unwrap();
    assert!(src.contains("notifyOnCreateOrder"), "hook must export notifyOnCreateOrder: {src}");
    assert!(src.contains("notifyOnCreateQuote"), "hook must export notifyOnCreateQuote: {src}");
}

#[test]
fn commerce_installs_jobs_run_route() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let path = root.join("apps/web/src/app/api/jobs/run/route.ts");
    assert!(path.exists(), "jobs/run/route.ts must be installed with commerce");
    let src = std::fs::read_to_string(&path).unwrap();
    assert!(src.contains("CF_CRON_SECRET"), "jobs run route must check CF_CRON_SECRET: {src}");
    assert!(src.contains("payload.jobs.run"), "jobs run route must call payload.jobs.run: {src}");
}

#[test]
fn orders_collection_has_after_change_hook() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let orders =
        std::fs::read_to_string(root.join("apps/web/src/payload/collections/Orders.ts")).unwrap();
    assert!(
        orders.contains("notifyOnCreateOrder"),
        "Orders must use notifyOnCreateOrder hook: {orders}"
    );
    assert!(
        orders.contains("afterChange"),
        "Orders must have afterChange hooks: {orders}"
    );
}

#[test]
fn quotes_collection_has_after_change_hook() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let quotes =
        std::fs::read_to_string(root.join("apps/web/src/payload/collections/Quotes.ts")).unwrap();
    assert!(
        quotes.contains("notifyOnCreateQuote"),
        "Quotes must use notifyOnCreateQuote hook: {quotes}"
    );
    assert!(
        quotes.contains("afterChange"),
        "Quotes must have afterChange hooks: {quotes}"
    );
}

#[test]
fn payload_config_registers_job_tasks() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    let cfg = std::fs::read_to_string(root.join("apps/web/payload.config.ts")).unwrap();
    assert!(cfg.contains("notifyNewOrderTask"), "config must import notifyNewOrderTask: {cfg}");
    assert!(cfg.contains("notifyNewQuoteTask"), "config must import notifyNewQuoteTask: {cfg}");
    assert!(cfg.contains("jobs:"), "config must register jobs: {cfg}");
}

#[test]
fn commerce_seed_messages_include_email_subjects() {
    let tmp = tempfile::tempdir().unwrap();
    let root = commerce_product(tmp.path());

    for locale in &["en", "sr"] {
        let msgs = std::fs::read_to_string(root.join(format!("messages/{locale}.json"))).unwrap();
        assert!(
            msgs.contains("newOrderNotificationSubject"),
            "[{locale}] messages must contain newOrderNotificationSubject: {msgs}"
        );
        assert!(
            msgs.contains("newQuoteNotificationSubject"),
            "[{locale}] messages must contain newQuoteNotificationSubject: {msgs}"
        );
    }
}
