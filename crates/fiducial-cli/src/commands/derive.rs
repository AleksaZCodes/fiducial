//! `fid derive [--check] [--pipeline <name>]` — run product pipelines.
//!
//! Discovers pipelines in `pipelines/*.toml` at the product root.
//! The built-in "types" pipeline runs `cargo test --features ts -p fiducial-wasm`
//! when `[spine] enabled = true` in `fiducial.toml`.
//!
//! `--check` mode: re-hashes outputs against `fiducial.lock [artifacts]` and
//! exits non-zero if any artifact is stale or missing. CI uses this.

use anyhow::{bail, Context, Result};
use std::{collections::BTreeMap, path::Path, process::Command};

use fiducial_eda::validate as validate_board_interface;
use fiducial_geometry::{BoardOutline, Side, ToleranceClass};
use fiducial_mesh::{
    enclosure_for, extrude_board, to_glb, to_stl_binary, Case, CaseParams, Cutout,
};

use crate::{
    config::{Config, SqlDialect, CONFIG_FILE},
    lock::{sha256_hex, short_hash, Lock, LOCK_FILE},
    pipeline::{self, Pipeline},
};

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(check: bool, pipeline_filter: Option<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = Config::find_root(&cwd)?;
    run_in(&root, check, pipeline_filter)
}

/// Derive inside a known product root.
///
/// Split out so `fid new` can leave the scaffold in a derived state. It cannot
/// call `run`, which resolves the root from the working directory — and a fresh
/// product whose artifacts have never been derived fails the `fid derive
/// --check` its own scaffolded CI runs, on the first commit, before anyone has
/// changed anything.
pub fn run_in(root: &Path, check: bool, pipeline_filter: Option<String>) -> Result<()> {
    let config_path = root.join(CONFIG_FILE);
    let lock_path = root.join(LOCK_FILE);

    let config = Config::load(&config_path)?;
    let mut lock = if lock_path.exists() {
        Lock::load(&lock_path)?
    } else {
        Lock::new()
    };

    let pipelines = pipeline::discover(root, &config)?;

    let to_run: Vec<&Pipeline> = match &pipeline_filter {
        Some(name) => {
            let found: Vec<_> = pipelines.iter().filter(|p| &p.name == name).collect();
            if found.is_empty() {
                bail!("no pipeline named `{name}` found in {}", root.display());
            }
            found
        }
        None => pipelines.iter().collect(),
    };

    if to_run.is_empty() {
        println!("✦ fid derive — no pipelines declared");
        println!("  Declare pipelines in `pipelines/*.toml` or enable [spine] in fiducial.toml.");
        return Ok(());
    }

    if check {
        run_check(&to_run, &lock, root)
    } else {
        run_derive(&to_run, root, &mut lock, &lock_path)
    }
}

// ── Derive (write) mode ───────────────────────────────────────────────────────

fn run_derive(
    pipelines: &[&Pipeline],
    root: &Path,
    lock: &mut Lock,
    lock_path: &Path,
) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let mut any_error = false;

    for pipeline in pipelines {
        print!("  ▶ {} ({})", pipeline.name, pipeline.executor);

        let working_dir = match &pipeline.working_dir {
            Some(rel) => root.join(rel),
            None => root.to_path_buf(),
        };

        match run_pipeline_command(pipeline, &working_dir) {
            Ok(_) => {
                println!(" ✓");
                let skip = outputs_not_applicable(pipeline, root);
                for out in &pipeline.outputs {
                    if skip.contains(out) {
                        // Stop tracking an artifact this configuration no
                        // longer derives, and say so if the old file is still
                        // sitting there. Removing it is not ours to do — it
                        // may have been applied to a real database — but
                        // leaving it unmentioned is how a product ships a
                        // migration nothing generates.
                        lock.artifacts.remove(out.as_str());
                        if root.join(out).exists() {
                            println!(
                                "      note: `{out}` is no longer derived \
                                 ([identity] storage = \"none\") but still exists"
                            );
                        }
                        continue;
                    }
                    let abs = root.join(out);
                    match std::fs::read(&abs) {
                        Ok(content) => {
                            lock.record_artifact(out, &content, &pipeline.name, version);
                        }
                        Err(e) => {
                            eprintln!("  ⚠ could not read output `{out}`: {e}");
                            any_error = true;
                        }
                    }
                }
            }
            Err(e) => {
                println!(" ✗");
                eprintln!("  error: {e}");
                any_error = true;
            }
        }
    }

    lock.save(lock_path)?;

    if any_error {
        bail!("one or more pipelines failed");
    }
    println!("\n✦ fid derive complete — fiducial.lock updated");
    Ok(())
}

// ── Check mode ────────────────────────────────────────────────────────────────

/// Outputs a pipeline declares that this product's configuration does not produce.
///
/// A pipeline's `outputs` list is written by the capability author, who cannot
/// know which of them a given product will want: `[identity] storage = "none"`
/// installs the authorization rule without deriving a table, so the migration
/// in that list is never written. Without this, `fid derive --check` demands an
/// artifact the declaration says should not exist — reporting a correctly
/// configured product as broken.
///
/// Kept as a lookup here rather than a field on `Pipeline` because it is a
/// question about *this product's config*, which a TOML file installed once
/// cannot answer.
pub(crate) fn outputs_not_applicable(pipeline: &Pipeline, root: &Path) -> Vec<String> {
    match pipeline.executor.as_str() {
        "fid-identity" => identity_outputs_not_applicable(pipeline, root),
        // The design capability is installed in every new product, because the
        // alternative to having a design system is having the default one. But
        // its output is a stylesheet, and a firmware-only product has nowhere
        // to put it: deriving there would create an orphan `apps/web/` tree
        // and then `--check` would demand it forever.
        //
        // So the output is not applicable until the app it belongs to exists.
        // Add one — `fid add app next` — and it starts deriving with no further
        // ceremony.
        "fid-design" => pipeline
            .outputs
            .iter()
            .filter(|o| !app_root_exists(root, o))
            .cloned()
            .collect(),
        _ => Vec::new(),
    }
}

/// Is there a real app at the root the output lives under?
///
/// `apps/web/src/app/tokens.css` → is `apps/web` an app? Two segments, because
/// that is the depth every `fid add app` target scaffolds at.
///
/// The test is a manifest, not a directory. The `design` capability installs
/// its own `apps/web/src/app/marks.css`, so the *directory* exists from the
/// moment the capability does — checking for it would mean this guard never
/// fires and a firmware-only product carries a stylesheet it has no way to
/// load. A `package.json` or `Cargo.toml` is what actually says "an app lives
/// here".
fn app_root_exists(root: &Path, output: &str) -> bool {
    let mut parts = Path::new(output).components();
    let (Some(a), Some(b)) = (parts.next(), parts.next()) else {
        // Not an app-shaped path at all — assume the author meant it.
        return true;
    };
    let app = root.join(a.as_os_str()).join(b.as_os_str());
    app.join("package.json").is_file() || app.join("Cargo.toml").is_file()
}

fn identity_outputs_not_applicable(pipeline: &Pipeline, root: &Path) -> Vec<String> {
    let Ok(config) = Config::load(&root.join(crate::config::CONFIG_FILE)) else {
        return Vec::new();
    };
    // `storage = "none"` derives no table at all, so every `.sql` output goes.
    if !config.identity.stores_grants() {
        return pipeline
            .outputs
            .iter()
            .filter(|o| {
                Path::new(o.as_str())
                    .extension()
                    .is_some_and(|e| e == "sql")
            })
            .cloned()
            .collect();
    }
    // The audit table is opt-in, so its migration is an output this product
    // does not produce unless it asked for one.
    if !config.identity.audit {
        return pipeline
            .outputs
            .iter()
            .filter(|o| o.ends_with("0002_audit.sql"))
            .cloned()
            .collect();
    }
    Vec::new()
}

/// What a deterministic executor would write for its first `.ts` output.
///
/// `None` for an executor that is not a pure function of committed
/// declarations — one that shells out to a tool, or reads something the lock
/// already tracks. Those cannot be re-run for free, and guessing at their
/// output would report noise as staleness.
///
/// The doc on [`outputs_with_moved_inputs`] explains why this exists at all.
/// It named generalizing beyond `fid-schema` as "a change worth making when a
/// second one needs it"; `fid-adapters` is that second one.
fn expected_output(pipeline: &Pipeline, root: &Path) -> Option<String> {
    match pipeline.executor.as_str() {
        "fid-schema" => {
            // A migration set that does not validate is reported by `fid
            // derive` with the reason. Repeating it here as "stale" would be
            // worse information, not more.
            let migrations = crate::schema::discover(root).ok()?;
            Some(crate::schema::render_manifest(&migrations))
        }
        // Same reasoning: a config that does not load, or an `[ai]` block that
        // does not validate, is `fid derive`'s error to report precisely.
        "fid-adapters" => {
            let config = Config::load(&root.join(crate::config::CONFIG_FILE)).ok()?;
            render_adapters_factory(&config).ok()
        }
        _ => None,
    }
}

/// Artifacts whose *inputs* moved, found by regenerating and comparing.
///
/// `fid derive --check` hashes declared outputs against `fiducial.lock`. That
/// catches a hand-edited artifact, and misses the opposite: adding
/// `migrations/0003_x.sql` and forgetting to re-run `fid derive` leaves a
/// manifest whose file is byte-identical to what the lock recorded, so the
/// check passes — and the migration silently never runs. That is precisely the
/// class of failure the migration system exists to prevent, so it cannot be
/// the one it ships with.
///
/// The same hole was open under `fid-adapters`, and nothing had noticed
/// because until `[ai] model` there was no declaration a product would edit
/// *often*: switching `database = "none"` to `"d1"` and forgetting to re-run
/// derive passed `--check` and shipped a Worker still constructing
/// `NoneDatabase`. A declaration nothing gates is documentation.
///
/// Both executors are pure functions of committed declarations, so the honest
/// check is to run them and compare — see [`expected_output`].
fn outputs_with_moved_inputs(pipeline: &Pipeline, root: &Path) -> Vec<String> {
    let Some(expected) = expected_output(pipeline, root) else {
        return Vec::new();
    };

    pipeline
        .outputs
        .iter()
        .filter(|out| {
            Path::new(out.as_str())
                .extension()
                .is_some_and(|e| e == "ts")
                && std::fs::read_to_string(root.join(out.as_str()))
                    .map(|actual| actual != expected)
                    .unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn run_check(pipelines: &[&Pipeline], lock: &Lock, root: &Path) -> Result<()> {
    let mut issues: Vec<String> = Vec::new();

    for pipeline in pipelines {
        for out in outputs_with_moved_inputs(pipeline, root) {
            issues.push(format!(
                "  {out}: stale — its inputs changed (run `fid derive`)"
            ));
        }
        let skip = outputs_not_applicable(pipeline, root);
        for out in &pipeline.outputs {
            if skip.contains(out) {
                continue;
            }
            let abs = root.join(out);
            match lock.artifacts.get(out.as_str()) {
                None => {
                    issues.push(format!(
                        "  {out}: not in fiducial.lock (run `fid derive` first)"
                    ));
                }
                Some(record) => match std::fs::read(&abs) {
                    Err(e) => {
                        issues.push(format!("  {out}: missing — {e}"));
                    }
                    Ok(content) => {
                        let actual = sha256_hex(&content);
                        if actual != record.hash {
                            issues.push(format!(
                                "  {out}: stale (lock:{} file:{}) — run `fid derive`",
                                short_hash(&record.hash),
                                short_hash(&actual),
                            ));
                        }
                    }
                },
            }
        }
    }

    if issues.is_empty() {
        println!("✦ fid derive --check — all artifacts fresh");
        Ok(())
    } else {
        eprintln!("✗ fid derive --check failed:\n{}", issues.join("\n"));
        bail!("stale artifacts detected");
    }
}

// ── Built-in fid-validate executor ───────────────────────────────────────────

/// Validate each output listed in the pipeline against its declared schema.
///
/// Supported schema names (pipeline `args[0]`):
///   - `board-interface` — validates JSON against the `BoardInterface` schema
///
/// Does not run any external process; validates in-process using `fiducial-eda`.
fn run_fid_validate(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let schema = pipeline
        .args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("board-interface");

    match schema {
        "board-interface" => {
            for out in &pipeline.outputs {
                let abs = working_dir.join(out);
                let json =
                    std::fs::read_to_string(&abs).with_context(|| format!("reading {out}"))?;
                validate_board_interface(&json).map_err(|e| anyhow::anyhow!("{out}: {e}"))?;
            }
            Ok(())
        }
        other => bail!("fid-validate: unknown schema `{other}` (supported: board-interface)"),
    }
}

// ── Built-in fid-mesh executor ───────────────────────────────────────────────

/// Parts the `fid-mesh` executor can generate, selected by output file stem.
const MESH_PARTS: &[&str] = &["case-base", "case-lid", "gasket", "case", "board", "tray"];

/// Generate case geometry declared by a `board.interface.json` outline.
///
/// `args[0]` is the path to the board interface JSON (default
/// `board/board.interface.json`).
///
/// Each output is addressed by **stem** and **extension**, which are
/// orthogonal: the stem picks the part, the extension picks the format. So
/// `enclosure/case-base.stl` and `enclosure/case-base.glb` are the same
/// geometry in two encodings.
///
/// | Stem | Part |
/// |---|---|
/// | `case-base` | base tray with the gasket groove |
/// | `case-lid` | lid with the compression tongue (print orientation) |
/// | `gasket` | gasket ring — **print in TPU** |
/// | `case` | exploded assembly, for rendering |
/// | `board` | the bare PCB, extruded |
/// | `tray` | simple open tray, no seal |
///
/// Connectors that declare a `mount` are punched through the base walls, sized
/// from their `type`'s body envelope. The declaration is validated before any
/// geometry is written: an opening that reaches the gasket groove fails the
/// pipeline rather than shipping a case that slices cleanly and leaks.
///
/// Runs in-process — no CAD tool required in CI.
fn run_fid_mesh(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let source = pipeline
        .args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("board/board.interface.json");

    let json = std::fs::read_to_string(working_dir.join(source))
        .with_context(|| format!("reading {source}"))?;
    let bi = validate_board_interface(&json).map_err(|e| anyhow::anyhow!("{source}: {e}"))?;

    let outline_decl = bi.outline.ok_or_else(|| {
        anyhow::anyhow!("{source} declares no `outline`; nothing for the mesh pipeline to derive")
    })?;

    // validate() already rejected unknown tolerance names and out-of-range
    // enclosure overrides, so this conversion cannot fail.
    let tolerance = match outline_decl.tolerance.as_str() {
        "resin" => ToleranceClass::Resin,
        "cnc" => ToleranceClass::Cnc,
        _ => ToleranceClass::Fdm,
    };
    let outline = BoardOutline::new(outline_decl.width_mm, outline_decl.height_mm)
        .with_thickness(outline_decl.thickness_mm)
        .with_tolerance(tolerance);

    let mut params = CaseParams::from_outline(&outline);
    if let Some(e) = &outline_decl.enclosure {
        if let Some(v) = e.headroom_mm {
            params.headroom_mm = v;
        }
        if let Some(v) = e.lid_thickness_mm {
            params.lid_thickness_mm = v;
        }
        if let Some(v) = e.gasket_width_mm {
            params.gasket_width_mm = v;
        }
        if let Some(v) = e.gasket_height_mm {
            params.gasket_height_mm = v;
        }
        if let Some(v) = e.gasket_compression {
            params.gasket_compression = v;
        }
        if let Some(v) = e.standoff_height_mm {
            params.standoff_height_mm = v;
        }
        if let Some(v) = e.standoff_size_mm {
            params.standoff_size_mm = v;
        }
        if let Some(v) = e.fastener_diameter_mm {
            params.fastener_diameter_mm = v;
        }
    }

    // Every connector that declares a mount becomes an opening. Size comes
    // from the connector's family unless the declaration overrides it —
    // validate() already rejected a mount whose family implies nothing, so
    // the envelope is present here.
    let mut cutouts: Vec<Cutout> = Vec::new();
    for c in &bi.connectors {
        let Some(m) = &c.mount else { continue };
        let (w, h) = m
            .envelope(&c.kind)
            .ok_or_else(|| anyhow::anyhow!("{source}: connector {} has no body envelope", c.id))?;
        let side = Side::from_name(&m.side)
            .ok_or_else(|| anyhow::anyhow!("{source}: connector {} has an unknown side", c.id))?;
        cutouts.push(
            Cutout::new(&c.id, side, m.offset_mm, w, h).with_z_offset(m.z_offset_mm.unwrap_or(0.0)),
        );
    }

    let case = Case::new(outline).with_params(params).with_cutouts(cutouts);

    // Validate before writing anything. A cutout that breaches the seal or
    // runs off its wall produces geometry a slicer accepts, so the only place
    // it can still be reported against the declaration is here.
    case.validate()
        .map_err(|e| anyhow::anyhow!("{source}: {e}"))?;

    for out in &pipeline.outputs {
        let path = Path::new(out);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("fid-mesh: output `{out}` has no file name"))?;

        let mesh = match stem {
            "case-base" => case.base(),
            "case-lid" => case.lid(),
            "gasket" => case.gasket(),
            "case" => case.exploded(),
            "board" => extrude_board(&outline),
            "tray" => enclosure_for(&outline),
            other => bail!(
                "fid-mesh: unknown part `{other}` in output `{out}` (expected one of: {})",
                MESH_PARTS.join(", ")
            ),
        };

        let bytes = match path.extension().and_then(|e| e.to_str()) {
            Some("stl") => to_stl_binary(&mesh),
            Some("glb") => to_glb(&mesh),
            _ => bail!("fid-mesh: unsupported output `{out}` (expected .stl or .glb)"),
        };

        let abs = working_dir.join(out);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&abs, bytes).with_context(|| format!("writing {out}"))?;
    }

    Ok(())
}

// ── Built-in fid-i18n executor ───────────────────────────────────────────────

/// Compare every locale catalog against the default, then generate typed keys.
///
/// Principle 1c made mechanical: a missing translation is a missing artifact, so
/// it fails here rather than rendering a key to a reader.
///
/// `args[0]` is the messages directory (default `messages`). Outputs are the
/// TypeScript modules to write — usually one.
fn run_fid_i18n(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    // The config owns two facts this executor needs: where the catalogs live,
    // and which locale is the fallback. Loaded once, up front.
    let config = Config::load(&working_dir.join(crate::config::CONFIG_FILE))
        .context("fid-i18n needs [i18n] in fiducial.toml")?;

    // `args[0]` overrides the declaration for an unusual layout; the
    // declaration is the default, so the directory is stated in one place.
    let dir_name = pipeline
        .args
        .first()
        .map(|s| s.as_str())
        .unwrap_or_else(|| config.i18n.messages_dir());
    let dir = working_dir.join(dir_name);

    if !dir.is_dir() {
        bail!(
            "fid-i18n: `{dir_name}/` does not exist.\n\
             Message catalogs live there, one JSON file per locale."
        );
    }

    // The catalogs on disk are the declaration; the locale set is read from
    // them rather than from config, so adding a file is all it takes.
    let mut catalogs: BTreeMap<String, crate::i18n::Catalog> = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).with_context(|| format!("reading {dir_name}/"))? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let Some(locale) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        catalogs.insert(locale.to_string(), crate::i18n::load_catalog(&path)?);
    }

    if catalogs.is_empty() {
        bail!("fid-i18n: no `{dir_name}/<locale>.json` catalogs found");
    }

    // The default locale is declared, not guessed from the directory listing.
    let default_locale = config.i18n.default_locale()?.to_string();

    if !catalogs.contains_key(&default_locale) {
        bail!(
            "fid-i18n: the default locale `{default_locale}` has no catalog.\n\
             Expected {dir_name}/{default_locale}.json"
        );
    }

    let findings = crate::i18n::compare(&catalogs, &default_locale)?;

    if findings.is_fatal() {
        let mut message = String::from("fid-i18n: the catalogs are not complete.\n");
        for (locale, keys) in &findings.missing {
            message.push_str(&format!(
                "\n  {dir_name}/{locale}.json is missing {} key(s):\n",
                keys.len()
            ));
            for key in keys.iter().take(10) {
                message.push_str(&format!("    {key}\n"));
            }
            if keys.len() > 10 {
                message.push_str(&format!("    … and {} more\n", keys.len() - 10));
            }
        }
        for (locale, keys) in &findings.placeholder_mismatch {
            message.push_str(&format!(
                "\n  {dir_name}/{locale}.json has {} key(s) whose placeholders \
                 differ from {default_locale}:\n",
                keys.len()
            ));
            for key in keys.iter().take(10) {
                message.push_str(&format!("    {key}\n"));
            }
            message.push_str(
                "    A placeholder in one locale and not the other means a reader\n\
                 \x20   sees a literal `{name}`, or loses the value entirely.\n",
            );
        }
        message.push_str(
            "\nA missing translation is a missing artifact, not a fallback \
             (MISSION.md 1c).\n",
        );
        bail!(message);
    }

    // Non-fatal findings are surfaced but do not stop the build — a legitimate
    // "Wi-Fi" must not block anyone.
    for (locale, keys) in &findings.untranslated {
        println!(
            "\n    ⚠ {locale}: {} value(s) identical to {default_locale} — likely untranslated",
            keys.len()
        );
        for key in keys.iter().take(5) {
            println!("      {key}");
        }
    }

    let rendered = crate::i18n::render_typescript(&catalogs, &default_locale)?;
    for out in &pipeline.outputs {
        let abs = working_dir.join(out);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating parent for `{out}`"))?;
        }
        std::fs::write(&abs, &rendered).with_context(|| format!("writing {out}"))?;
    }

    Ok(())
}

// ── Built-in fid-brand executor ───────────────────────────────────────────────

/// One declaration in, a handful of static artifacts out.
///
/// `[brand]` in `fiducial.toml` is the declaration; each output is addressed by
/// **file name**, the same way `fid-mesh` addresses parts by stem — the name
/// says which artifact, the pipeline's `outputs` says where it lands.
///
/// | File name | Derives |
/// |---|---|
/// | `robots.txt` | crawler policy pointing at the sitemap |
/// | `sitemap.xml` | the domain root — a real route list is a router's declaration, not brand's |
/// | `site.webmanifest` | app name/colours, pointing at the generated favicon |
/// | `favicon.svg` | a vector mark from the trading name's initials — no rasterizer required |
/// | `organization.jsonld` | a `schema.org` `Organization` record |
///
/// Rendering itself is pure and lives in `crate::brand`; this function is only
/// I/O — reading the declaration, and writing what it renders.
fn run_fid_brand(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let config = Config::load(&working_dir.join(crate::config::CONFIG_FILE))
        .context("fid-brand needs [brand] in fiducial.toml")?;
    let brand = &config.brand;

    if brand.is_empty() {
        bail!(
            "fid-brand: [brand] is not declared in fiducial.toml.\n\
             Add [brand] with legal_name, trading_name, domain and contact_email \
             (the `brand` capability seeds a placeholder — `fid add brand`)."
        );
    }
    brand.validate()?;

    for out in &pipeline.outputs {
        let path = Path::new(out);
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("fid-brand: output `{out}` has no file name"))?;

        let content = match name {
            "robots.txt" => crate::brand::render_robots(&brand.domain),
            "sitemap.xml" => crate::brand::render_sitemap(&brand.domain),
            "site.webmanifest" => crate::brand::render_manifest(
                &brand.trading_name,
                brand.short_name.as_deref().unwrap_or(&brand.trading_name),
                brand.primary_color(),
                brand.background_color(),
            ),
            // The declared file wins over the generated initials, and the
            // output stays derived either way — so `fid derive --check` still
            // guards it and editing the copy in place still fails, which is the
            // property that makes the override safe rather than a hole.
            "favicon.svg" => match brand.favicon.as_deref() {
                Some(src) => {
                    let from = working_dir.join(src);
                    std::fs::read_to_string(&from).with_context(|| {
                        format!(
                            "fid-brand: [brand] favicon = \"{src}\" could not be read.\n\
                             The path is relative to the product root, and the file must exist \
                             before `fid derive` runs. Remove the key to fall back to the \
                             generated mark."
                        )
                    })?
                }
                None => crate::brand::render_favicon_svg(
                    &brand.trading_name,
                    brand.primary_color(),
                    brand.background_color(),
                ),
            },
            "organization.jsonld" => crate::brand::render_jsonld(
                &brand.legal_name,
                &brand.trading_name,
                &brand.domain,
                &brand.contact_email,
            ),
            "brand.ts" => crate::brand::render_brand_ts(
                &brand.legal_name,
                &brand.trading_name,
                brand.short_name.as_deref().unwrap_or(&brand.trading_name),
                &brand.domain,
                &brand.contact_email,
                brand.primary_color(),
                brand.background_color(),
            ),
            other => bail!(
                "fid-brand: unknown output `{other}` (supported: robots.txt, sitemap.xml, \
                 site.webmanifest, favicon.svg, organization.jsonld, brand.ts)"
            ),
        };

        let abs = working_dir.join(out);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating parent for `{out}`"))?;
        }
        std::fs::write(&abs, content).with_context(|| format!("writing {out}"))?;
    }

    Ok(())
}

// ── Built-in fid-deploy executor ─────────────────────────────────────────────

/// Generate `wrangler.toml` from `[adapters]` + `[deploy]`.
///
/// **Which bindings a Worker needs is derivable, and was being hand-copied.**
/// `[adapters] database = "d1"` already says the product wants D1, and
/// `D1Database` already reads a fixed `env.DB` — so the `[[d1_databases]]`
/// block with `binding = "DB"` is a *derivation* of facts already declared,
/// not a seventh place to state them.
///
/// It was being hand-copied, and it was already wrong: the `worker-cloudflare`
/// template shipped commented examples binding `MY_DB` and `MY_BUCKET`, while
/// every adapter reads `DB` and `BUCKET`. Following the template's own example
/// produced a Worker that threw at the first query — the exact class of defect
/// this platform exists to delete, sitting inside the platform.
///
/// What the account knows and nothing can derive — a D1 database id, a bucket
/// name, a route — stays declared in `[deploy]`, and the bindings are
/// generated around it.
fn run_fid_deploy(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let config = Config::load(&working_dir.join(crate::config::CONFIG_FILE))
        .context("fid-deploy needs [deploy] in fiducial.toml")?;

    let vendor = config.adapters.get("deploy").unwrap_or("none");
    if vendor != "cloudflare" {
        bail!(
            "fid-deploy: [adapters] deploy = \"{vendor}\" — this executor \
             generates a Cloudflare `wrangler.toml`. Set deploy = \"cloudflare\", \
             or remove pipelines/deploy.toml if this product ships elsewhere."
        );
    }
    config.deploy.validate(&config.adapters)?;

    let product = &config.product.name;
    let d = &config.deploy;
    let name = if d.name.is_empty() {
        format!("{product}-worker")
    } else {
        d.name.clone()
    };
    let main = if d.main.is_empty() {
        "src/index.ts"
    } else {
        &d.main
    };

    let mut out = String::new();
    out.push_str("# generated by `fid derive` — do not edit\n");
    out.push_str("# source of truth: [deploy] and [adapters] in fiducial.toml\n");
    out.push_str("#\n");
    out.push_str("# Bindings are DERIVED from [adapters]: selecting a vendor there is\n");
    out.push_str("# what puts its binding here, under the fixed name the adapter reads.\n");
    out.push_str("# Editing this file by hand fails `fid derive --check`.\n\n");
    out.push_str(&format!("name = \"{name}\"\n"));
    out.push_str(&format!("main = \"{main}\"\n"));
    out.push_str(&format!(
        "compatibility_date = \"{}\"\n",
        d.compatibility_date
    ));
    if !d.compatibility_flags.is_empty() {
        let flags: Vec<String> = d
            .compatibility_flags
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect();
        out.push_str(&format!("compatibility_flags = [{}]\n", flags.join(", ")));
    }
    if !d.route.is_empty() {
        out.push_str(&format!("\nroute = \"{}\"\n", d.route));
    }

    out.push_str("\n[vars]\n");
    out.push_str(&format!("FIDUCIAL_PRODUCT = \"{product}\"\n"));

    // ── Derived bindings ────────────────────────────────────────────────
    // Each block appears exactly when [adapters] selected the vendor that
    // reads it, under the binding name that vendor's adapter class reads.
    if config.adapters.get("database") == Some("d1") {
        let db_name = if d.d1_database_name.is_empty() {
            product.clone()
        } else {
            d.d1_database_name.clone()
        };
        out.push_str("\n# [adapters] database = \"d1\" → D1Database reads env.DB\n");
        out.push_str("[[d1_databases]]\n");
        out.push_str("binding = \"DB\"\n");
        out.push_str(&format!("database_name = \"{db_name}\"\n"));
        out.push_str(&format!("database_id = \"{}\"\n", d.d1_database_id));
    }

    if config.adapters.get("storage") == Some("r2") {
        out.push_str("\n# [adapters] storage = \"r2\" → R2Storage reads env.BUCKET\n");
        out.push_str("[[r2_buckets]]\n");
        out.push_str("binding = \"BUCKET\"\n");
        out.push_str(&format!("bucket_name = \"{}\"\n", d.r2_bucket_name));
    }

    if config.adapters.get("queue") == Some("cloudflare-queues") {
        out.push_str(
            "\n# [adapters] queue = \"cloudflare-queues\" → CloudflareQueue reads env.QUEUE\n",
        );
        out.push_str("[[queues.producers]]\n");
        out.push_str("binding = \"QUEUE\"\n");
        out.push_str(&format!("queue = \"{}\"\n", d.queue_name));
    }

    // Secrets are named, never written. `wrangler secret put` is the only
    // place a secret's value belongs; a generated file in the repository is
    // the one place it must never be.
    //
    // RESEND_API_KEY is required by both `email = "resend"` and
    // `newsletter = "resend"`, so we collect into a Vec and deduplicate
    // (preserving first-occurrence order) before rendering — otherwise a
    // product selecting both would emit the line twice.
    let mut secrets: Vec<&str> = Vec::new();
    if config.adapters.get("botProtection") == Some("turnstile") {
        secrets.push("TURNSTILE_SECRET_KEY");
    }
    if config.adapters.get("email") == Some("resend") {
        secrets.push("RESEND_API_KEY");
    }
    if config.adapters.get("newsletter") == Some("resend") {
        secrets.push("RESEND_API_KEY");
        secrets.push("RESEND_AUDIENCE_ID");
    }
    if config.adapters.get("ai") == Some("openrouter") {
        secrets.push("OPENROUTER_API_KEY");
    }
    if config.adapters.get("auth") == Some("supabase") {
        secrets.push("SUPABASE_URL");
        secrets.push("SUPABASE_ANON_KEY");
    }
    if config.adapters.get("database") == Some("supabase") {
        secrets.push("SUPABASE_DB_URL");
    }
    if config.adapters.get("storage") == Some("supabase-storage") {
        secrets.push("SUPABASE_URL");
        secrets.push("SUPABASE_SERVICE_ROLE_KEY");
    }
    // Deduplicate while preserving first-occurrence order.
    let mut seen = std::collections::HashSet::new();
    secrets.retain(|s| seen.insert(*s));
    if !secrets.is_empty() {
        out.push_str("\n# Secrets this product's adapters read. Set each with:\n");
        for s in &secrets {
            out.push_str(&format!("#   wrangler secret put {s}\n"));
        }
        out.push_str("# Never written here — a generated file in the repository is the\n");
        out.push_str("# one place a secret's value must not be.\n");
    }

    let target = pipeline
        .outputs
        .first()
        .ok_or_else(|| anyhow::anyhow!("fid-deploy: pipeline declares no outputs"))?;
    let abs = working_dir.join(target);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent for `{target}`"))?;
    }
    std::fs::write(&abs, out).with_context(|| format!("writing {target}"))?;
    Ok(())
}

// ── Built-in fid-identity executor ───────────────────────────────────────────

/// Every principal kind a grant may be held by.
///
/// `anonymous` is deliberately absent: `can()` refuses an unidentified
/// principal, so a grant to anonymous could never be honoured. Leaving it out
/// of the `CHECK` means the database refuses to store a row that the rule
/// would ignore — the same decision, enforced twice, at the only two places
/// it can be made.
const GRANT_PRINCIPAL_KINDS: &[&str] = &["user", "device", "service"];

/// Every resource kind a grant may cover. Mirrors `Resource` in
/// `fiducial-identity`; `platform` stores an empty id.
const GRANT_RESOURCE_KINDS: &[&str] = &["user", "device", "platform"];

/// Every role, weakest first. Mirrors `Role` in `fiducial-identity`.
const GRANT_ROLES: &[&str] = &["viewer", "member", "admin", "owner"];

/// Generate the grants table from the identity model.
///
/// The schema is a **derivation of the types**: the `CHECK` lists are exactly
/// the variants `Principal`, `Resource` and `Role` define, so adding a role
/// and forgetting the migration is not a thing that can happen quietly —
/// `fid derive --check` fails instead.
///
/// ## Two dialects, because "SQL" is not one language
///
/// The first version of this generated SQLite only, and used `GLOB` for the
/// zero-sentinel check — which is SQLite syntax that Postgres rejects with a
/// plain syntax error. A product on Supabase or Neon could not run its own
/// generated migration. The dialect is now **derived from the database vendor
/// the product already selected** (`d1` → SQLite; `supabase`/`neon`/`postgres`
/// → Postgres), because that fact is already declared and asking for it twice
/// is how the two get out of step.
fn run_fid_identity(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let config = Config::load(&working_dir.join(crate::config::CONFIG_FILE))
        .context("fid-identity needs [identity] in fiducial.toml")?;
    config.identity.validate(&config.adapters)?;

    // `storage = "none"` installs the rule without the table. `can()` still
    // works — it takes grants as an argument and does not care where they came
    // from — so the TypeScript module is still generated, saying so.
    if !config.identity.stores_grants() {
        if let Some(ts_target) = output_with_extension(pipeline, "ts") {
            write_output(working_dir, ts_target, &render_identity_module_none())?;
        }
        return Ok(());
    }

    let table = config.identity.table_name();
    let dialect = config.identity.resolve_dialect(&config.adapters)?;

    let quoted = |values: &[&str]| -> String {
        values
            .iter()
            .map(|v| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    // The one genuine dialect difference in the table itself: "this string
    // contains a character that is not '0'". SQLite spells it GLOB, Postgres
    // spells it with a POSIX regex, and neither understands the other.
    let not_all_zero = match dialect {
        SqlDialect::Sqlite => format!("{table}_nonzero CHECK (principal_id GLOB '*[^0]*')"),
        SqlDialect::Postgres => format!("{table}_nonzero CHECK (principal_id ~ '[^0]')"),
    };
    // Postgres has a real timestamp type; SQLite does not.
    let timestamp_type = match dialect {
        SqlDialect::Sqlite => "TEXT",
        SqlDialect::Postgres => "TIMESTAMPTZ",
    };
    // `expires_at` holds milliseconds since the epoch, which is ~1.79e12 and
    // does not fit Postgres's 4-byte INTEGER — it errors with "integer out of
    // range" on every insert. SQLite's INTEGER is up to 8 bytes, so the same
    // DDL worked there and the defect was invisible until a real Postgres
    // server ran it. The same shape as `GLOB`: SQL is not one language, and a
    // schema asserted as text is not a schema that has been tried.
    let epoch_millis_type = match dialect {
        SqlDialect::Sqlite => "INTEGER",
        SqlDialect::Postgres => "BIGINT",
    };

    let mut out = String::new();
    out.push_str("-- generated by `fid derive` — do not edit\n");
    out.push_str("-- source of truth: the identity model in `fiducial-identity`,\n");
    out.push_str("-- plus [identity] in fiducial.toml for the table name.\n");
    out.push_str(&format!(
        "--\n-- Dialect: {} (derived from [adapters] database).\n",
        dialect.as_str()
    ));
    out.push_str("--\n-- A grant is one row: who, over what, how much.\n");
    out.push_str("--\n-- The CHECK lists below are the `Principal`, `Resource` and `Role`\n");
    out.push_str("-- variants themselves. `anonymous` is absent on purpose: `can()`\n");
    out.push_str("-- refuses an unidentified principal, so the database refuses to\n");
    out.push_str("-- store a row the rule would ignore.\n\n");

    out.push_str(&format!("CREATE TABLE IF NOT EXISTS {table} (\n"));
    out.push_str(&format!(
        "  principal_kind TEXT NOT NULL CHECK (principal_kind IN ({})),\n",
        quoted(GRANT_PRINCIPAL_KINDS)
    ));
    out.push_str("  principal_id   TEXT NOT NULL,\n");
    out.push_str(&format!(
        "  resource_kind  TEXT NOT NULL CHECK (resource_kind IN ({})),\n",
        quoted(GRANT_RESOURCE_KINDS)
    ));
    out.push_str("  -- '' for a platform-scoped grant, which covers every resource.\n");
    out.push_str("  resource_id    TEXT NOT NULL,\n");
    out.push_str(&format!(
        "  role           TEXT NOT NULL CHECK (role IN ({})),\n",
        quoted(GRANT_ROLES)
    ));
    out.push_str(&format!("  granted_at     {timestamp_type} NOT NULL,\n"));
    out.push_str("  -- NULL is a permanent grant. A grant that has to be remembered to\n");
    out.push_str("  -- be revoked is one that is not revoked, so a support engineer's\n");
    out.push_str("  -- access carries its own end date. Milliseconds since the epoch,\n");
    out.push_str("  -- matching `Timestamp` in both languages.\n");
    out.push_str(&format!("  expires_at     {epoch_millis_type},\n"));
    out.push_str("  -- Who delegated this grant, if anyone. A delegated grant is worth\n");
    out.push_str("  -- exactly what the delegator's own authority is worth AT THE TIME\n");
    out.push_str("  -- IT IS EVALUATED — so revoking a manager revokes everything they\n");
    out.push_str("  -- handed out, without anyone having to go and find it. Stored, not\n");
    out.push_str("  -- flattened into the grant, for exactly that reason.\n");
    out.push_str(&format!(
        "  delegated_by_kind TEXT CHECK (delegated_by_kind IS NULL OR delegated_by_kind IN ({})),\n",
        quoted(GRANT_PRINCIPAL_KINDS)
    ));
    out.push_str("  delegated_by_id   TEXT,\n");
    out.push_str("  -- Both halves of a delegator, or neither. Half a principal is not\n");
    out.push_str("  -- one, and a row with an id and no kind would be silently ignored.\n");
    out.push_str(&format!(
        "  CONSTRAINT {table}_delegator_whole CHECK (\n    \
         (delegated_by_kind IS NULL AND delegated_by_id IS NULL)\n    \
         OR (delegated_by_kind IS NOT NULL AND delegated_by_id IS NOT NULL)\n  ),\n"
    ));
    out.push_str("  -- An all-zero id is the uninitialized sentinel at every level: an\n");
    out.push_str("  -- unprovisioned device must not hold a grant as \"device zero\".\n");
    out.push_str(&format!("  CONSTRAINT {not_all_zero},\n"));
    out.push_str("  -- One role per principal per resource: granting again replaces.\n");
    out.push_str(
        "  PRIMARY KEY (principal_kind, principal_id, resource_kind, resource_id)\n);\n\n",
    );

    out.push_str("-- `grantsFor(principal)` — the read on the authorization path.\n");
    out.push_str(&format!(
        "CREATE INDEX IF NOT EXISTS {table}_by_principal\n  ON {table} (principal_kind, principal_id);\n\n"
    ));
    out.push_str("-- `grantsOn(resource)` — the read a sharing UI makes.\n");
    out.push_str(&format!(
        "CREATE INDEX IF NOT EXISTS {table}_by_resource\n  ON {table} (resource_kind, resource_id);\n"
    ));

    if config.identity.rls {
        out.push_str(&render_grant_policies(
            table,
            config.identity.current_user_expr(),
        ));
    }

    // Two outputs, matched by extension rather than by position: the schema,
    // and the two facts a product would otherwise retype to construct a store
    // against it. Retyping them is exactly how a Postgres migration ends up
    // queried with SQLite placeholders.
    let sql_target = pipeline
        .outputs
        .iter()
        .find(|o| o.ends_with("0001_grants.sql"))
        .map(String::as_str)
        .or_else(|| output_with_extension(pipeline, "sql"))
        .ok_or_else(|| anyhow::anyhow!("fid-identity: no `.sql` output declared"))?;
    write_output(working_dir, sql_target, &out)?;

    // A SECOND migration, not an edit to the first. Editing an applied
    // migration is the one thing `Migrator.apply()` refuses outright, and a
    // generator that exempted itself from the rule it generates for would be
    // producing exactly the drift it warns about.
    if config.identity.audit {
        if let Some(target) = pipeline
            .outputs
            .iter()
            .find(|o| o.ends_with("0002_audit.sql"))
        {
            write_output(
                working_dir,
                target,
                &render_audit_migration(table, dialect, timestamp_type),
            )?;
        }
    }

    if let Some(ts_target) = output_with_extension(pipeline, "ts") {
        write_output(
            working_dir,
            ts_target,
            &render_identity_module(table, dialect),
        )?;
    }
    Ok(())
}

/// `fid-schema` — the ordered migration manifest, from `migrations/*.sql`.
///
/// The SQL files are the declaration; this is the derivation that makes them
/// applicable somewhere with no filesystem. Validation (ordering, unique
/// numbers, names that are actually migrations) happens in `schema::discover`,
/// so a set that cannot be applied safely fails here rather than at deploy.
fn run_fid_schema(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let migrations = crate::schema::discover(working_dir)?;

    let target = output_with_extension(pipeline, "ts")
        .ok_or_else(|| anyhow::anyhow!("fid-schema: no `.ts` output declared for the manifest"))?;
    write_output(
        working_dir,
        target,
        &crate::schema::render_manifest(&migrations),
    )
}

/// The pipeline's output with this extension, if it declares one.
fn output_with_extension<'a>(pipeline: &'a Pipeline, ext: &str) -> Option<&'a str> {
    pipeline
        .outputs
        .iter()
        .map(String::as_str)
        .find(|o| Path::new(o).extension().is_some_and(|e| e == ext))
}

fn write_output(working_dir: &Path, target: &str, contents: &str) -> Result<()> {
    let abs = working_dir.join(target);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent for `{target}`"))?;
    }
    std::fs::write(&abs, contents).with_context(|| format!("writing {target}"))
}

/// The facts a product needs to construct `SqlGrantStore` against the schema
/// generated above — as TypeScript, so it imports them instead of typing them
/// again.
///
/// This exists because one of them is easy to get silently wrong. A product on
/// Supabase gets a Postgres migration, and a `SqlGrantStore` left at its
/// SQLite default sends `?` placeholders at it: a syntax error on every query,
/// found at runtime. Both facts are already declared — `[identity] table` and
/// `[adapters] database` — so this is them arriving in the language that uses
/// them, not a third declaration.
///
/// A marker template rather than `format!`: the output is full of braces, and
/// escaping every one of them makes the generated TypeScript unreadable in the
/// generator, which is where anyone edits it.
/// The append-only log of authorization decisions.
///
/// `explain()` produces the record and this is where it goes. The columns are
/// the `Decision` fields, so a reason the rule can return and the table cannot
/// store is a `CHECK` failure rather than a row that silently loses why.
///
/// Append-only by convention and by grant, not by trigger: SQLite and Postgres
/// disagree about how to forbid an UPDATE, and a log whose immutability is
/// enforced differently on each vendor is one whose guarantee nobody can state.
fn render_audit_migration(table: &str, dialect: SqlDialect, timestamp_type: &str) -> String {
    let reasons = AUDIT_REASONS
        .iter()
        .map(|r| format!("'{r}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let bool_type = match dialect {
        SqlDialect::Sqlite => "INTEGER",
        SqlDialect::Postgres => "BOOLEAN",
    };
    let audit = format!("{table}_audit");

    format!(
        "-- generated by `fid derive` — do not edit\n\
         -- source of truth: `Decision` and `Reason` in `fiducial-identity`.\n\
         --\n\
         -- Every authorization decision, with WHY. The verdict alone is not an\n\
         -- audit trail: reconstructing the reason afterwards from the grants table\n\
         -- is guesswork, because the table has moved on by the time anyone reads\n\
         -- the log.\n\
         --\n\
         -- A SECOND migration rather than an edit to 0001. Editing an applied\n\
         -- migration is the one thing the runner refuses outright.\n\
         \n\
         CREATE TABLE IF NOT EXISTS {audit} (\n  \
           id             INTEGER PRIMARY KEY{autoinc},\n  \
           decided_at     {timestamp_type} NOT NULL,\n  \
           principal_kind TEXT NOT NULL,\n  \
           principal_id   TEXT NOT NULL,\n  \
           action         TEXT NOT NULL CHECK (action IN ('read', 'write', 'admin')),\n  \
           resource_kind  TEXT NOT NULL,\n  \
           resource_id    TEXT NOT NULL,\n  \
           allowed        {bool_type} NOT NULL,\n  \
           -- The `Reason` variants themselves. A reason the rule can return and\n  \
           -- this cannot store fails here rather than losing why.\n  \
           reason         TEXT NOT NULL CHECK (reason IN ({reasons}))\n\
         );\n\
         \n\
         -- The query an audit answers: what happened to this principal, latest first.\n\
         CREATE INDEX IF NOT EXISTS {audit}_principal_idx\n  \
           ON {audit} (principal_kind, principal_id, decided_at);\n\
         \n\
         -- And the other one: who was refused, and why.\n\
         CREATE INDEX IF NOT EXISTS {audit}_denied_idx\n  \
           ON {audit} (allowed, decided_at);\n",
        autoinc = match dialect {
            SqlDialect::Sqlite => "",
            SqlDialect::Postgres => " GENERATED BY DEFAULT AS IDENTITY",
        },
    )
}

/// Every `Reason` the rule can return. Mirrors `Reason` in `fiducial-identity`.
const AUDIT_REASONS: &[&str] = &[
    "granted",
    "device_reading_itself",
    "not_identified",
    "no_grant",
    "role_too_weak",
    "expired",
    "no_clock",
    "delegator_lacks_authority",
    "delegation_too_deep",
];

/// The module a product gets when it wants the rule and not the table.
///
/// It exports something rather than nothing on purpose: a product importing
/// `./identity.generated.js` should get a clear compile error pointing at the
/// declaration, not a module-not-found that says nothing about why.
fn render_identity_module_none() -> String {
    r#"// generated by `fid derive` — do not edit
//
// `[identity] storage = "none"` — this product installs the authorization
// RULE without deriving a table for it. No migration is generated.
//
// `can()` is unaffected. It takes grants as an argument and does not care
// where they came from, which is what makes `none` a real configuration
// rather than a disabled one:
//
//   import { can, MemoryGrantStore } from "@fiducial/identity";
//
//   const store = new MemoryGrantStore(grantsFromYourOwnSource);
//   if (can(principal, "read", resource, await store.grantsFor(principal))) …
//
// To derive a table instead, set `[identity] storage = "sql"` in
// fiducial.toml and re-run `fid derive`.

/** `[identity] storage`, carried into TypeScript so a consumer can branch. */
export const grantStorage = "none" as const;

/** Constructing a SQL store is a mistake this product's declaration forbids. */
export function grantStore(): never {
  throw new Error(
    '[identity] storage = "none": no grants table is derived for this product. ' +
      'Use MemoryGrantStore, or set storage = "sql" in fiducial.toml.',
  );
}
"#
    .to_string()
}

fn render_identity_module(table: &str, dialect: SqlDialect) -> String {
    const TEMPLATE: &str = r#"// generated by `fid derive` — do not edit
//
// What `migrations/0001_grants.sql` needs to be talked to. Both facts are
// declared once — `[identity] table` and `[adapters] database` — and the
// migration beside this file was generated from the same two.
//
// The dialect is here because getting it wrong is silent until runtime: a
// Postgres table queried with SQLite's `?` placeholders is a syntax error on
// every statement.
//
//   import { grantStore } from "./identity.generated.js";
//
//   const store = grantStore(env.DB);
//   const grants = await store.grantsFor(who);

import { SqlGrantStore } from "@fiducial/identity";
import type {
  GrantDatabase,
  SqlDialect,
  SqlGrantStoreOptions,
} from "@fiducial/identity";

/** `[identity] storage`, carried into TypeScript so a consumer can branch. */
export const grantStorage = "sql" as const;

/** `[identity] table`. */
export const grantsTable = "__TABLE__";

/** Resolved from `[adapters] database`: the dialect the schema was generated
 * for, and the one its placeholders have to match. */
export const sqlDialect: SqlDialect = "__DIALECT__";

export const grantStoreOptions: SqlGrantStoreOptions = {
  table: grantsTable,
  dialect: sqlDialect,
};

/** A `SqlGrantStore` over the schema this derivation generated. */
export function grantStore(db: GrantDatabase): SqlGrantStore {
  return new SqlGrantStore(db, grantStoreOptions);
}
"#;
    TEMPLATE
        .replace("__TABLE__", table)
        .replace("__DIALECT__", dialect.as_str())
}

/// Row-level security for the grants table — Postgres defence in depth.
///
/// `can()` remains *the* rule: it is the one both languages share, the one the
/// conformance vectors check, and the only one firmware can run. RLS cannot
/// replace it — a `no_std` device reaches no Postgres engine. What RLS adds is
/// that an application bug stops being a data breach: a missed check in a
/// handler cannot read or forge grant rows the database will not hand over.
///
/// These policies are **derived from the same model**, which is what makes a
/// second enforcement point safe rather than a second source of truth.
///
/// The rules:
/// - a principal may read its own grants;
/// - a principal may read grants over a resource it administers;
/// - only an admin or owner of a resource may write grants over it.
///
/// ## Why the check is a function and not an inlined `EXISTS`
///
/// The obvious spelling — `EXISTS (SELECT 1 FROM grants …)` inside a policy
/// *on* `grants` — re-enters the policy to evaluate itself. Postgres refuses
/// it at query time, not at `CREATE POLICY` time:
///
/// ```text
/// ERROR:  infinite recursion detected in policy for relation "grants"
/// ```
///
/// So the lookup goes in a `SECURITY DEFINER` function, which runs as the
/// table's owner. The owner is not subject to the table's policies, so the
/// inner read completes and the recursion never starts. That is also why this
/// does **not** emit `FORCE ROW LEVEL SECURITY`: forcing policies onto the
/// owner would put the helper back inside the loop it exists to break.
///
/// `SET search_path = ''` with fully-qualified names is mandatory rather than
/// tidy: a `SECURITY DEFINER` function with an inherited search path lets any
/// caller shadow the objects it names and run their own code as the owner.
///
/// `current_user_sql` is normalized to this platform's id form: `auth.uid()`
/// yields a hyphenated UUID, and `principal_id` holds 32 lowercase hex
/// characters (see `userIdFromUuid`).
fn render_grant_policies(table: &str, current_user_sql: &str) -> String {
    // The current user as a `principal_id`: hyphens dropped, lowercased.
    let me = format!("replace(lower(({current_user_sql})::text), '-', '')");
    let administers = format!("public.{table}_administers(resource_kind, resource_id)");

    format!(
        "\n\
         -- ── Row-level security ───────────────────────────────────────────\n\
         --\n\
         -- Defence in depth, not the rule itself. `can()` stays the rule: it\n\
         -- is the one both languages share, the one the conformance vectors\n\
         -- check, and the only one firmware can run — a no_std device reaches\n\
         -- no Postgres engine. What these add is that a missed check in a\n\
         -- handler cannot read or forge grant rows the database will not hand\n\
         -- over.\n\
         --\n\
         -- Derived from the same model as `can()`, which is what makes a\n\
         -- second enforcement point safe rather than a second source of truth.\n\
         --\n\
         -- Current user expression: {current_user_sql}\n\
         --\n\
         -- These bind for roles that are not the table's owner — Supabase's\n\
         -- `authenticated` and `anon`. A connection as the owner (or as\n\
         -- Supabase's `service_role`) bypasses them, which is what lets a\n\
         -- trusted server-side path administer grants.\n\
         \n\
         ALTER TABLE {table} ENABLE ROW LEVEL SECURITY;\n\
         \n\
         -- \"Does the current user administer this resource?\"\n\
         --\n\
         -- This has to be a SECURITY DEFINER function rather than an inlined\n\
         -- EXISTS: a policy on {table} that reads {table} re-enters itself,\n\
         -- and Postgres rejects the query with \"infinite recursion detected\n\
         -- in policy\". Running as the table's owner — who is not subject to\n\
         -- its policies — is what breaks the loop. That is also why FORCE ROW\n\
         -- LEVEL SECURITY is deliberately absent: forcing policies onto the\n\
         -- owner would put this function back inside the recursion.\n\
         --\n\
         -- `search_path = ''` with fully-qualified names is required, not\n\
         -- tidiness: a SECURITY DEFINER function that inherits the caller's\n\
         -- search path lets the caller shadow what it names and run code as\n\
         -- the owner.\n\
         --\n\
         -- A `platform`-scoped grant covers every resource, which is what the\n\
         -- second arm of the OR is for.\n\
         CREATE OR REPLACE FUNCTION public.{table}_administers(res_kind text, res_id text)\n\
         \u{20} RETURNS boolean\n\
         \u{20} LANGUAGE sql\n\
         \u{20} STABLE\n\
         \u{20} SECURITY DEFINER\n\
         \u{20} SET search_path = ''\n\
         AS $fn$\n\
         \u{20} SELECT EXISTS (\n\
         \u{20}   SELECT 1 FROM public.{table} AS g\n\
         \u{20}    WHERE g.principal_kind = 'user'\n\
         \u{20}      AND g.principal_id = {me}\n\
         \u{20}      AND g.role IN ('admin', 'owner')\n\
         \u{20}      AND (\n\
         \u{20}            (g.resource_kind = res_kind AND g.resource_id = res_id)\n\
         \u{20}         OR g.resource_kind = 'platform'\n\
         \u{20}          )\n\
         \u{20} )\n\
         $fn$;\n\
         \n\
         -- A principal reads its own grants; an administrator of a resource\n\
         -- reads every grant over it (the sharing UI's query).\n\
         DROP POLICY IF EXISTS {table}_select ON {table};\n\
         CREATE POLICY {table}_select ON {table}\n\
         \u{20} FOR SELECT USING (\n\
         \u{20}   (principal_kind = 'user' AND principal_id = {me})\n\
         \u{20}   OR {administers}\n\
         \u{20} );\n\
         \n\
         -- Only an admin or owner of a resource may grant over it. Note this\n\
         -- refuses self-promotion: the lookup asks for a role you already\n\
         -- hold, so writing yourself an `owner` row requires already being one.\n\
         DROP POLICY IF EXISTS {table}_insert ON {table};\n\
         CREATE POLICY {table}_insert ON {table}\n\
         \u{20} FOR INSERT WITH CHECK ({administers});\n\
         \n\
         DROP POLICY IF EXISTS {table}_update ON {table};\n\
         CREATE POLICY {table}_update ON {table}\n\
         \u{20} FOR UPDATE USING ({administers}) WITH CHECK ({administers});\n\
         \n\
         DROP POLICY IF EXISTS {table}_delete ON {table};\n\
         CREATE POLICY {table}_delete ON {table}\n\
         \u{20} FOR DELETE USING ({administers});\n"
    )
}

// ── Built-in fid-adapters executor ───────────────────────────────────────────

/// `([adapters] key, AdapterSet field name)` for every contract that has a
/// runtime factory slot. Almost always identical — `errors` is the one
/// exception, because the `[adapters]` key predates the `Diagnostics` trait
/// name and renaming a `fiducial.toml` key is a breaking change for no
/// benefit. Adding a contract with a runtime trait means adding one row here;
/// nothing else in this function changes.
const ADAPTER_SLOTS: &[(&str, &str)] = &[
    ("database", "database"),
    ("storage", "storage"),
    ("email", "email"),
    ("errors", "diagnostics"),
    ("botProtection", "botProtection"),
    ("queue", "queue"),
    ("newsletter", "newsletter"),
    ("ai", "ai"),
];

/// Generate adapter factory code from `[adapters]` in `fiducial.toml`.
///
/// Reads the declared vendor for each contract in `ADAPTER_SLOTS`, then
/// writes a TypeScript factory file at the single output path declared in the
/// pipeline (`outputs[0]`). The generated file imports the right class for
/// each slot and exports a `createAdapters(env)` function.
///
/// When a vendor's class is not yet implemented (i.e. still `none`), the
/// factory imports `None*` from `@fiducial/adapters`. When a real vendor lands
/// (e.g. `d1`), the factory imports `D1Database` instead — no callers change.
///
/// `auth` is **not** one of `ADAPTER_SLOTS` and does not join `AdapterSet` —
/// it gets a second, separate `createAuth(env, ctx)` export in the same
/// file instead. Every other contract's vendor is env-scoped (a binding or an
/// API key lives on `env` for the life of the Worker); a real `Auth`
/// implementation is request-scoped, needing a session store bound to that
/// request's cookies or `Authorization` header. Folding it into
/// `createAdapters(env)`'s uniform `new {Class}(env)` shape would either
/// silently drop that argument or corrupt the shape for the other five slots.
/// See `packages/adapters/src/auth.ts`'s module doc.
fn run_fid_adapters(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let config = Config::load(&working_dir.join(crate::config::CONFIG_FILE))
        .context("fid-adapters needs [adapters] in fiducial.toml")?;
    let content = render_adapters_factory(&config)?;

    let out = pipeline
        .outputs
        .first()
        .ok_or_else(|| anyhow::anyhow!("fid-adapters: pipeline declares no outputs"))?;

    let abs = working_dir.join(out);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("creating parent for `{out}`"))?;
    }
    std::fs::write(&abs, content).with_context(|| format!("writing {out}"))?;
    Ok(())
}

/// The factory file's text, as a pure function of `fiducial.toml`.
///
/// Separated from the write so `fid derive --check` can regenerate and compare
/// without touching the working tree — see [`expected_output`]. Two code paths
/// producing "what the factory should say" is exactly the second declaration
/// this platform exists to delete.
fn render_adapters_factory(config: &Config) -> Result<String> {
    config.ai.validate(&config.adapters)?;
    let adapters = &config.adapters;

    let mut imports = Vec::with_capacity(ADAPTER_SLOTS.len() + 1);
    let mut fields = Vec::with_capacity(ADAPTER_SLOTS.len());
    for (toml_key, field_name) in ADAPTER_SLOTS {
        let vendor = adapters.get(toml_key).unwrap_or("none");
        imports.push(vendor_ts_import(toml_key, vendor));
        let class = vendor_ts_class(toml_key, vendor);
        let extra = vendor_extra_args(toml_key, vendor, config);
        fields.push(format!("    {field_name}: new {class}(env{extra}),"));
    }

    let auth_vendor = adapters.get("auth").unwrap_or("none");
    let auth_import = vendor_ts_import("auth", auth_vendor);
    let auth_class = vendor_ts_class("auth", auth_vendor);

    let content = format!(
        "// generated by `fid derive` — do not edit\n\
         // source of truth: [adapters] in fiducial.toml\n\
         //\n\
         // To change a vendor: edit fiducial.toml and re-run `fid derive`.\n\
         // CI gate: `fid derive --check` fails if this file is stale.\n\
         \n\
         {}\n\
         {auth_import}\n\
         import type {{ AdapterSet }} from \"@fiducial/adapters\";\n\
         import type {{ Auth, AuthSessionContext }} from \"@fiducial/adapters/auth\";\n\
         \n\
         // eslint-disable-next-line @typescript-eslint/no-explicit-any\n\
         export function createAdapters(env?: any): AdapterSet {{\n\
         \u{20}\u{20}return {{\n\
         {}\n\
         \u{20}\u{20}}};\n\
         }}\n\
         \n\
         // `auth` is request-scoped (a session store, not just `env`) — see\n\
         // this file's own generator doc comment in derive.rs for why it is\n\
         // not part of AdapterSet/createAdapters above. Construct one\n\
         // createAuth(env, ctx) call per request.\n\
         // eslint-disable-next-line @typescript-eslint/no-explicit-any\n\
         export function createAuth(env: any, ctx: AuthSessionContext): Auth {{\n\
         \u{20}\u{20}return new {auth_class}(env, ctx);\n\
         }}\n",
        imports.join("\n"),
        fields.join("\n"),
    );

    Ok(content)
}

/// TypeScript import line for a contract + vendor pair.
///
/// Until a vendor ships an implementation, it falls through to the `none`
/// import from `@fiducial/adapters`. As each vendor lands, add it here *and*
/// to `implementations` in `adapter::CONTRACTS` — the two must move together,
/// or `fid doctor` calls a selection valid that derive cannot actually wire.
fn vendor_ts_import(contract: &str, vendor: &str) -> String {
    let (class, path) = vendor_ts_class_and_path(contract, vendor);
    format!("import {{ {class} }} from \"{path}\";")
}

/// TypeScript class name for instantiation in the factory body.
fn vendor_ts_class(contract: &str, vendor: &str) -> String {
    vendor_ts_class_and_path(contract, vendor).0
}

/// Constructor arguments after `env`, as literal TypeScript source.
///
/// Empty for every vendor but one. `new {Class}(env)` is the shape
/// `createAdapters` relies on, and `auth` was kept out of `AdapterSet`
/// entirely rather than bend it — but that was about a *lifetime*: a session
/// store is request-scoped and cannot be built from `env` at Worker scope.
///
/// A declared model is not that. It is a constant `fid derive` already knows,
/// so writing it into the generated file as a literal is the same move every
/// other derived fact makes, and it keeps the slot env-scoped. The alternative
/// — a wrangler `[vars]` entry the class reads off `env` — puts the fact in a
/// file only Cloudflare products generate, and this contract is reached from
/// Next.js and SvelteKit server routes too.
fn vendor_extra_args(contract: &str, vendor: &str, config: &Config) -> String {
    match (contract, vendor) {
        ("ai", "openrouter") => format!(", {}", ts_string(&config.ai.model)),
        _ => String::new(),
    }
}

/// A TypeScript string literal for a value that came from `fiducial.toml`.
///
/// Escaped rather than interpolated: a declaration is product-supplied text,
/// and a stray quote in it would produce a generated file that does not parse.
fn ts_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// `(ClassName, import-path)` for a contract + vendor pair.
fn vendor_ts_class_and_path(contract: &str, vendor: &str) -> (String, String) {
    let none_class = match contract {
        "database" => ("NoneDatabase", "@fiducial/adapters/database"),
        "storage" => ("NoneStorage", "@fiducial/adapters/storage"),
        "email" => ("NoneEmail", "@fiducial/adapters/email"),
        "errors" => ("NoneDiagnostics", "@fiducial/adapters/diagnostics"),
        "botProtection" => ("NoneBotProtection", "@fiducial/adapters/bot-protection"),
        "queue" => ("NoneQueue", "@fiducial/adapters/queue"),
        "newsletter" => ("NoneNewsletter", "@fiducial/adapters/newsletter"),
        "ai" => ("NoneAi", "@fiducial/adapters/ai"),
        "auth" => ("NoneAuth", "@fiducial/adapters/auth"),
        _ => ("NoneDatabase", "@fiducial/adapters"),
    };

    // Real vendor implementations land here. Falling back to `none_class` for
    // an unrecognized (contract, vendor) pair is intentional: selecting a
    // candidate that has no implementation yet is caught by `fid doctor`;
    // derive still writes a file so the build stays green.
    let real = match (contract, vendor) {
        ("database", "d1") => Some(("D1Database", "@fiducial/adapters/database")),
        ("storage", "r2") => Some(("R2Storage", "@fiducial/adapters/storage")),
        ("email", "resend") => Some(("ResendEmail", "@fiducial/adapters/email")),
        ("botProtection", "turnstile") => Some(("Turnstile", "@fiducial/adapters/bot-protection")),
        ("queue", "cloudflare-queues") => Some(("CloudflareQueue", "@fiducial/adapters/queue")),
        ("newsletter", "resend") => Some(("ResendNewsletter", "@fiducial/adapters/newsletter")),
        ("ai", "openrouter") => Some(("OpenRouterAi", "@fiducial/adapters/ai")),
        ("auth", "supabase") => Some(("SupabaseAuth", "@fiducial/adapters/auth")),
        ("database", "supabase") => Some(("SupabaseDatabase", "@fiducial/adapters/database")),
        ("storage", "supabase-storage") => Some(("SupabaseStorage", "@fiducial/adapters/storage")),
        _ => None,
    };

    match real {
        Some((class, path)) => (class.to_string(), path.to_string()),
        None => (none_class.0.to_string(), none_class.1.to_string()),
    }
}

// ── Built-in fid-design executor ──────────────────────────────────────────────

/// Derive the theme stylesheet from `design-system.md`.
///
/// The declaration is `args[0]` (default `design-system.md`) and the single
/// output is the generated token stylesheet. The product's own `globals.css`
/// imports it and keeps everything that is not a token — the base layer, the
/// marks layer, the commentary. A generator that owned the whole stylesheet
/// would delete all of that on the next run.
///
/// The palette is measured before it is written: `fid:contrast` declares the
/// pairs and their minimums, and a palette that misses one fails here. That is
/// the difference between a design system that documents a rule and one that
/// holds it.
fn run_fid_design(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let source = pipeline
        .args
        .first()
        .map(String::as_str)
        .unwrap_or("design-system.md");

    let src_path = working_dir.join(source);
    let md = std::fs::read_to_string(&src_path).with_context(|| {
        format!(
            "fid-design: cannot read `{source}`.\n  \
             It is the declaration — install it with `fid add design`."
        )
    })?;

    let system = crate::design::parse(&md)
        .with_context(|| format!("fid-design: `{source}` does not parse"))?;

    let pairs = system
        .check_contrast()
        .context("fid-design: the declared palette does not meet its own requirements")?;

    let [out] = pipeline.outputs.as_slice() else {
        bail!(
            "fid-design: expected exactly one output (the token stylesheet), got {}",
            pipeline.outputs.len()
        );
    };

    // Declared, checked, but with nowhere to land yet. The palette above was
    // still measured — the declaration is worth policing before there is an app
    // to apply it to, which is the whole point of writing it first.
    if !app_root_exists(working_dir, out) {
        print!(" [{pairs} contrast pairs ok; no app yet — `{out}` not written]");
        return Ok(());
    }

    let css = system.generate_css(source);
    let abs = working_dir.join(out);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&abs, css).with_context(|| format!("writing {}", abs.display()))?;

    // Said out loud because a silent check is one nobody trusts: the whole
    // point of measuring in the pipeline is that the number is real.
    print!(" [{pairs} contrast pairs ok]");
    Ok(())
}

// ── Built-in fid-legal executor ───────────────────────────────────────────────

/// Derive localized legal page content from `[legal]`, `[brand]`, and `[i18n]`.
///
/// `[legal]` declares the jurisdiction, data protection email, and cookie
/// categories. `[brand]` provides the entity name and domain. `[i18n]` provides
/// the locale list. All three must be configured before this pipeline can run.
///
/// Output: a single TypeScript file `src/generated/legal.ts` (or wherever
/// `pipelines/legal.toml` points) with:
/// - `LegalPage` type union
/// - `LegalCatalog` type (`Record<LegalPage, { title, body }>`)
/// - One constant per locale and a `legalCatalogs` map keyed by locale string
///
/// The generated file starts with a GDPR checklist comment naming every
/// decision a human must still make — read it after the first `fid derive`.
fn run_fid_legal(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let config = Config::load(&working_dir.join(crate::config::CONFIG_FILE))
        .context("fid-legal needs [legal], [brand], and [i18n] in fiducial.toml")?;
    let legal = &config.legal;

    if legal.is_empty() {
        bail!(
            "fid-legal: [legal] is not declared in fiducial.toml.\n\
             Add [legal] with jurisdiction, data_protection_email, and cookie_categories \
             (the `legal` capability seeds a placeholder — `fid add legal`)."
        );
    }
    legal.validate()?;

    if config.i18n.is_empty() {
        bail!(
            "fid-legal: [i18n] is not configured in fiducial.toml.\n\
             The legal pipeline generates localized content for every declared locale.\n\
             Add [i18n] with locales and default (the `i18n` capability — `fid add i18n`)."
        );
    }
    let default_locale = config.i18n.default_locale()?.to_string();
    let locales = config.i18n.locales.clone();

    if config.brand.is_empty() {
        bail!(
            "fid-legal: [brand] is not configured in fiducial.toml.\n\
             The legal pipeline reads the entity name, domain, and contact from [brand].\n\
             Add [brand] with legal_name, trading_name, domain and contact_email \
             (the `brand` capability — `fid add brand`)."
        );
    }
    config.brand.validate()?;

    let rendered = crate::legal::render_legal_ts(legal, &config.brand, &locales, &default_locale)?;

    for out in &pipeline.outputs {
        let abs = working_dir.join(out);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating parent for `{out}`"))?;
        }
        std::fs::write(&abs, &rendered).with_context(|| format!("writing {out}"))?;
    }

    Ok(())
}

// ── Command execution ─────────────────────────────────────────────────────────

fn run_pipeline_command(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let (program, base_args, extra_args): (&str, Vec<&str>, Vec<&str>) =
        match pipeline.executor.as_str() {
            "cargo-test" => (
                "cargo",
                vec!["test"],
                pipeline.args.iter().map(|s| s.as_str()).collect(),
            ),
            "shell" => {
                if pipeline.args.is_empty() {
                    bail!("shell executor requires at least one arg (the command)");
                }
                (
                    pipeline.args[0].as_str(),
                    pipeline.args[1..].iter().map(|s| s.as_str()).collect(),
                    Vec::new(),
                )
            }
            "fid-validate" => return run_fid_validate(pipeline, working_dir),
            "fid-mesh" => return run_fid_mesh(pipeline, working_dir),
            "fid-i18n" => return run_fid_i18n(pipeline, working_dir),
            "fid-brand" => return run_fid_brand(pipeline, working_dir),
            "fid-deploy" => return run_fid_deploy(pipeline, working_dir),
            "fid-identity" => return run_fid_identity(pipeline, working_dir),
            "fid-adapters" => return run_fid_adapters(pipeline, working_dir),
            "fid-schema" => return run_fid_schema(pipeline, working_dir),
            "fid-legal" => return run_fid_legal(pipeline, working_dir),
            "fid-design" => return run_fid_design(pipeline, working_dir),
            other => bail!(
                "unknown executor `{other}` \
                 (supported: cargo-test, shell, fid-validate, fid-mesh, fid-i18n, fid-brand, \
                 fid-adapters, fid-deploy, fid-identity, fid-schema, fid-legal, fid-design)"
            ),
        };

    let status = Command::new(program)
        .args(&base_args)
        .args(&extra_args)
        .current_dir(working_dir)
        .status()
        .with_context(|| format!("spawning `{program}`"))?;

    if !status.success() {
        bail!("pipeline `{}` exited with {}", pipeline.name, status);
    }
    Ok(())
}
