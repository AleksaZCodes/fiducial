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
    config::{Config, CONFIG_FILE},
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
                for out in &pipeline.outputs {
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

fn run_check(pipelines: &[&Pipeline], lock: &Lock, root: &Path) -> Result<()> {
    let mut issues: Vec<String> = Vec::new();

    for pipeline in pipelines {
        for out in &pipeline.outputs {
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
                brand.primary_color(),
                brand.background_color(),
            ),
            "favicon.svg" => crate::brand::render_favicon_svg(
                &brand.trading_name,
                brand.primary_color(),
                brand.background_color(),
            ),
            "organization.jsonld" => crate::brand::render_jsonld(
                &brand.legal_name,
                &brand.trading_name,
                &brand.domain,
                &brand.contact_email,
            ),
            other => bail!(
                "fid-brand: unknown output `{other}` (supported: robots.txt, sitemap.xml, \
                 site.webmanifest, favicon.svg, organization.jsonld)"
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

// ── Built-in fid-adapters executor ───────────────────────────────────────────

/// Generate adapter factory code from `[adapters]` in `fiducial.toml`.
///
/// Reads the declared vendor for each contract, then writes a TypeScript factory
/// file at the single output path declared in the pipeline (`outputs[0]`). The
/// generated file imports the right class for each slot and exports a
/// `createAdapters(env)` function.
///
/// When a vendor's class is not yet implemented (i.e. still `none`), the
/// factory imports `None*` from `@fiducial/adapters`. When a real vendor lands
/// (e.g. `d1`), the factory imports `D1Database` instead — no callers change.
fn run_fid_adapters(pipeline: &Pipeline, working_dir: &Path) -> Result<()> {
    let config = Config::load(&working_dir.join(crate::config::CONFIG_FILE))
        .context("fid-adapters needs [adapters] in fiducial.toml")?;
    let adapters = &config.adapters;

    let db_vendor = adapters.get("database").unwrap_or("none");
    let storage_vendor = adapters.get("storage").unwrap_or("none");
    let email_vendor = adapters.get("email").unwrap_or("none");
    let errors_vendor = adapters.get("errors").unwrap_or("none");

    let db_import = vendor_ts_import("database", db_vendor);
    let storage_import = vendor_ts_import("storage", storage_vendor);
    let email_import = vendor_ts_import("email", email_vendor);
    let errors_import = vendor_ts_import("errors", errors_vendor);

    let db_class = vendor_ts_class("database", db_vendor);
    let storage_class = vendor_ts_class("storage", storage_vendor);
    let email_class = vendor_ts_class("email", email_vendor);
    let errors_class = vendor_ts_class("errors", errors_vendor);

    let content = format!(
        "// generated by `fid derive` — do not edit\n\
         // source of truth: [adapters] in fiducial.toml\n\
         //\n\
         // To change a vendor: edit fiducial.toml and re-run `fid derive`.\n\
         // CI gate: `fid derive --check` fails if this file is stale.\n\
         \n\
         {db_import}\n\
         {storage_import}\n\
         {email_import}\n\
         {errors_import}\n\
         import type {{ AdapterSet }} from \"@fiducial/adapters\";\n\
         \n\
         // eslint-disable-next-line @typescript-eslint/no-explicit-any\n\
         export function createAdapters(env?: any): AdapterSet {{\n\
         {}  return {{\n\
         {}    database:    new {db_class}(env),\n\
         {}    storage:     new {storage_class}(env),\n\
         {}    email:       new {email_class}(env),\n\
         {}    diagnostics: new {errors_class}(env),\n\
         {}  }};\n\
         }}\n",
        "  ", "  ", "  ", "  ", "  ", "  ",
    );

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

/// `(ClassName, import-path)` for a contract + vendor pair.
fn vendor_ts_class_and_path(contract: &str, vendor: &str) -> (String, String) {
    let none_class = match contract {
        "database" => ("NoneDatabase", "@fiducial/adapters/database"),
        "storage" => ("NoneStorage", "@fiducial/adapters/storage"),
        "email" => ("NoneEmail", "@fiducial/adapters/email"),
        "errors" => ("NoneDiagnostics", "@fiducial/adapters/diagnostics"),
        _ => ("NoneDatabase", "@fiducial/adapters"),
    };

    // Real vendor implementations land here. Falling back to `none_class` for
    // an unrecognized (contract, vendor) pair is intentional: selecting a
    // candidate that has no implementation yet is caught by `fid doctor`;
    // derive still writes a file so the build stays green.
    let real = match (contract, vendor) {
        ("database", "d1") => Some(("D1Database", "@fiducial/adapters/database")),
        ("storage", "r2") => Some(("R2Storage", "@fiducial/adapters/storage")),
        _ => None,
    };

    match real {
        Some((class, path)) => (class.to_string(), path.to_string()),
        None => (none_class.0.to_string(), none_class.1.to_string()),
    }
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
            "fid-adapters" => return run_fid_adapters(pipeline, working_dir),
            other => bail!(
                "unknown executor `{other}` \
                 (supported: cargo-test, shell, fid-validate, fid-mesh, fid-i18n, fid-brand, fid-adapters)"
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
