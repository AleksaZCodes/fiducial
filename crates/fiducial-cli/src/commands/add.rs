//! `fid add` — add an app, module, firmware target, or capability.
//!
//! With a structural subcommand (`app`, `module`, `firmware`) this scaffolds a
//! specific kind of target. With a bare capability id it delegates to the
//! capability installer, which writes templates, activates guard rules, and
//! writes the SKILL.md to `.claude/skills/`.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use std::env;

use crate::{
    capability,
    config::{Config, CONFIG_FILE},
};

// ── Subcommand tree ──────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
pub enum AddTarget {
    /// Add a web or native application
    #[command(
        long_about = "\
Scaffold a web or native application target.

Installs the corresponding platform capability and writes all template files
into the product. After installation, run `fid capability check` to verify.",
        after_long_help = "\
TARGETS
  next      Next.js 15 (App Router) web app     → apps/web/
  svelte    SvelteKit web app                    → apps/web/
  tauri     Tauri 2 desktop + mobile             → apps/desktop/
  worker    Cloudflare Worker / Durable Object   → apps/worker/
  mobile    Tauri mobile standalone              → apps/mobile/ (not available yet)

EXAMPLES
  fid add app next
  fid add app tauri
  fid add app worker"
    )]
    App {
        /// Target: next | svelte | tauri | worker | mobile
        #[arg(
            value_name = "TARGET",
            long_help = "Application type to scaffold. Run `fid add app --help` for the full list."
        )]
        target: String,
    },

    /// Add a bounded-context module (Rust + TS)
    #[command(
        long_about = "\
Scaffold a bounded-context module: a Rust crate + TypeScript package pair that
lives in the host workspace and can be imported by apps and firmware.

Modules are the idiomatic unit for shared business logic — protocol codecs,
state machines, domain rules — that must be correct across multiple runtimes.",
        after_long_help = "\
EXAMPLES
  fid add module payments     bounded context for payment flows
  fid add module auth         authentication / session logic
  fid add module telemetry    device telemetry pipeline

STATUS
  Not available yet. `SHIPPED.md` in the platform repository records what is
  planned; this help does not restate a schedule it would only get wrong."
    )]
    Module {
        /// Module name in kebab-case (e.g. `payments`, `auth`)
        #[arg(value_name = "NAME")]
        name: String,
    },

    /// Copy a UI component into apps/web/src/components/ui/
    #[command(
        long_about = "\
Copy a Fiducial UI component into this product's web app.

Components are copied into `apps/web/src/components/ui/` and become yours —
no runtime dependency on a Fiducial package. Edit freely; `fid upgrade` will
offer upstream changes via 3-way merge.

Base UI (@base-ui-components/react) is used for accessible complex components
(dialog, popover, menu). Run `pnpm add @base-ui-components/react` after adding
the dialog component.",
        after_long_help = "\
COMPONENTS
  button   Button with 4 variants (primary, secondary, ghost, destructive)
  card     Card, CardHeader, CardTitle, CardContent, CardFooter
  badge    Badge with 4 variants (default, secondary, destructive, outline)
  dialog   Accessible dialog (Base UI React / native <dialog> Svelte)

FRAMEWORKS
  react    (default) copies .tsx + .css
  svelte   copies .svelte

EXAMPLES
  fid add component button
  fid add component dialog --framework svelte
  fid add component card --framework react"
    )]
    Component {
        /// Component name: button | card | badge | dialog
        #[arg(value_name = "NAME")]
        name: String,
        /// Framework: react (default) | svelte
        #[arg(long, default_value = "react")]
        framework: String,
    },

    /// Add the EDA pipeline capability (atopile → KiCad → board.interface.json)
    #[command(
        long_about = "\
Add the EDA pipeline capability to this product.

Installs the `eda` capability: writes board/main.ato (atopile source),
board/board.interface.json (seed output), and pipelines/eda.toml
(fid derive pipeline config).

After installation:
  1. Edit board/main.ato with your schematic.
  2. Run `atopile build` to generate KiCad files + board.interface.json.
  3. Run `fid derive` to validate and record the artifact hash.
  4. CI runs `fid derive --check` to catch staleness.",
        after_long_help = "\
WORKFLOW
  atopile build    Compile .ato → KiCad + board.interface.json
  fid derive       Validate board.interface.json + record hash in fiducial.lock
  fid derive --check  CI check: fails if board.interface.json is stale

EXAMPLES
  fid add eda"
    )]
    Eda,

    /// Add localization — JSON catalogs, typed keys, and the freshness gate
    #[command(
        long_about = "\
Make this product localized by construction.

Installs `messages/<locale>.json` catalogs, a `fid-i18n` pipeline that generates
a typed MessageKey union from them, and a skill telling agents the rules.

After this, a missing translation FAILS `fid derive --check` rather than
rendering a key to a reader — MISSION.md principle 1c: a user-visible string is
a fact, declared once, with every locale a derivation that must exist.

Seeds `en` and `sr`. Add locales by dropping in another JSON file and listing it
under `[i18n]` in fiducial.toml.",
        after_long_help = "\
EXAMPLE:
    fid add i18n
    # edit messages/*.json
    fid derive          # regenerates the typed keys
    fid derive --check  # fails when a locale is missing a key"
    )]
    I18n,

    /// Add brand — one declaration, favicon/manifest/sitemap/robots/JSON-LD
    #[command(
        long_about = "\
Make this product's identity a declaration instead of scattered files.

Installs `[brand]` in fiducial.toml (legal name, trading name, domain, contact
email, two colours) and a `fid-brand` pipeline that derives robots.txt,
sitemap.xml, site.webmanifest, favicon.svg and organization.jsonld from it.

Seeds placeholder text so the pipeline does not fail on the first `fid
derive` — replace every field before deriving for real.",
        after_long_help = "\
EXAMPLE:
    fid add brand
    # edit [brand] in fiducial.toml
    fid derive          # writes robots.txt, sitemap.xml, site.webmanifest,
                         # favicon.svg, organization.jsonld
    fid derive --check  # fails if any is missing or stale"
    )]
    Brand,

    /// Add Cloudflare deploy config, derived from [adapters]
    #[command(
        long_about = "\
Install the deploy capability.

Generates `apps/worker/wrangler.toml` from `[adapters]` + `[deploy]`. Which
bindings a Worker needs is already declared — `[adapters] database = \"d1\"`
says the product wants D1, and `D1Database` reads a fixed `env.DB` — so the
binding blocks are derived rather than hand-copied. `[deploy]` holds only
what the Cloudflare account knows: a database id, a bucket name, a route.

Takes ownership of `apps/worker/wrangler.toml`: it becomes a derived
artifact, regenerated on every `fid derive` and gated by `--check`.",
        after_long_help = "\
EXAMPLE:
    fid add deploy
    # set [adapters] deploy = \"cloudflare\", then fill the ids in [deploy]
    fid derive          # writes apps/worker/wrangler.toml
    fid derive --check  # fails if it is stale or hand-edited
    wrangler deploy"
    )]
    Deploy,

    /// Add grant storage — the permission rows `can()` reads
    #[command(
        long_about = "\
Install the identity capability.

Auth answers who you are; a grant answers what you may do. A grant is one row
— principal, resource, role — and this capability is where those rows live.

Generates `migrations/0001_grants.sql` from the identity model: the table's
CHECK constraints are the Principal, Resource and Role variants themselves, so
adding a role and forgetting the migration fails `fid derive --check` rather
than drifting quietly.

Not a migrations system — it generates the first table only.",
        after_long_help = "\
EXAMPLE:
    fid add identity
    fid derive          # writes migrations/0001_grants.sql
    fid derive --check  # fails if the model changed and this did not"
    )]
    Identity,

    /// Add schema migrations — ordered, idempotent, checked for drift
    #[command(
        long_about = "\
Install the migrations capability.

SQL applied to this product's database. NOT the codemod migrations `fid
upgrade` applies to source files — they share a word and nothing else.

`migrations/NNNN_slug.sql` is the declaration. `fid derive` generates
`src/migrations.generated.ts`: the migrations in order, each with its SQL
embedded and hashed. Embedded because the runner has no filesystem — a Worker
applying migrations at deploy time cannot open a file — and hashed so a
migration edited after it was applied is detectable.

Apply it with the runner in @fiducial/adapters, which goes through the
`database` contract and so works on every vendor that contract has.

Generation fails rather than producing a set that cannot be applied safely:
two files sharing a number have no order, and a `.sql` file not named
`NNNN_slug.sql` is a migration that silently never runs.",
        after_long_help = "\
EXAMPLE:
    fid add migrations
    # write migrations/0002_add_index.sql
    fid derive          # regenerates src/migrations.generated.ts
    fid derive --check  # fails if the manifest is stale

WHAT THE RUNNER ENFORCES
    Never edit an applied migration. Environments that ran it keep the old
    schema, new ones get the new one, and neither can tell they disagree.
    `Migrator.apply()` refuses to run at all while that is true. Add a new
    migration instead."
    )]
    Migrations,

    /// Add localized legal pages — privacy, terms, cookies, and more
    #[command(
        long_about = "\
Make this product's legal pages a declaration instead of hand-written files.

Installs `[legal]` in fiducial.toml (jurisdiction, data protection email, cookie
categories) and a `fid-legal` pipeline that derives a typed TypeScript catalog of
legal page content for every declared locale.

Depends on `brand` (for entity name, domain, contact) and `i18n` (for the locale
set). Install both before running `fid derive`.

Legal copy is rendered per language: English and Serbian templates ship, and a
locale with no templates is emitted under a loud UNTRANSLATED banner rather than
silently served as English.

Seeds jurisdiction = \"EU\" and data_protection_email = \"privacy@example.com\" —
replace the email before deriving for real.

GDPR compliance is a legal state, not a code state. The generated file carries a
checklist comment naming every decision a human must still make.",
        after_long_help = "\
EXAMPLE:
    fid add legal
    # edit [legal] in fiducial.toml — replace data_protection_email
    fid derive          # writes src/generated/legal.ts
    fid derive --check  # fails if it is missing or stale"
    )]
    Legal,

    /// Add cross-platform adapter contracts (database, storage, email, diagnostics)
    #[command(
        long_about = "\
Install the adapters capability.

Adds `fiducial-adapters` (Rust) and `@fiducial/adapters` (TypeScript) contracts
to the product and wires `fid derive` to generate `src/adapters.generated.ts`
from the `[adapters]` declaration in fiducial.toml.

After installing, declare your vendor choices in fiducial.toml:

    [adapters]
    database = \"none\"   # swap for d1, supabase, neon, postgres when ready
    storage  = \"none\"   # swap for r2, s3, supabase-storage
    email    = \"none\"   # swap for resend, ses
    errors   = \"none\"   # swap for sentry, workers-analytics

Then run `fid derive` to regenerate the factory file.",
        after_long_help = "\
EXAMPLE
    fid add adapters
    fid derive          # writes src/adapters.generated.ts
    fid derive --check  # CI gate — fails if the factory is stale"
    )]
    Adapters,

    /// Add firmware support for a microcontroller target
    #[command(
        long_about = "\
Scaffold firmware for a specific microcontroller family.

Installs the `firmware-<target>` capability: writes the firmware workspace
structure, configures the embedded toolchain, and adds the relevant guard rules.",
        after_long_help = "\
TARGETS
  rp2040     Raspberry Pi RP2040 — Embassy + probe-rs      → firmware/rp2040/
  rp2350     Raspberry Pi RP2350 — Embassy + probe-rs      → firmware/rp2350/
  stm32      STM32 family — Embassy HAL                    → firmware/stm32/
  nrf52      nRF52 family — Embassy HAL                    → firmware/nrf52/  (not available yet)

EXAMPLES
  fid add firmware rp2040
  fid add firmware rp2350"
    )]
    Firmware {
        /// Board target: rp2040 | rp2350 | stm32 | nrf52
        #[arg(value_name = "TARGET")]
        target: String,
    },

    /// Install a capability by id, from the built-ins or an external source
    #[command(
        name = "capability",
        long_about = "\
Install a capability that is not one of the named targets above.

A capability need not be compiled into `fid`. With --from, it is resolved from
a directory or a git repository and installed exactly as a built-in one is —
same derivation, same conformance checks, same lock entry. Shipping a
capability does not require releasing the CLI.",
        after_long_help = "\
EXAMPLES
  fid add capability eda
  fid add capability stripe --from ./capabilities/stripe
  fid add capability stripe --from git:https://github.com/acme/fid-stripe
  fid add capability stripe --from git:https://github.com/acme/caps#v1.2.0::stripe

SOURCES
  <path>                        a directory whose name is the capability id
  git:<url>                     a repository's default branch
  git:<url>#<rev>               a tag, branch or commit
  git:<url>#<rev>::<subdir>     one capability inside a repository of several

A git source is always pinned in fiducial.lock at the commit it resolved to,
never at the branch name — so the same lock installs the same capability."
    )]
    Capability {
        /// Capability id, which must match the source directory's name
        #[arg(value_name = "ID")]
        id: String,
        /// Where to resolve it from; omit for a built-in
        #[arg(long, value_name = "SOURCE")]
        from: Option<String>,
    },
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

pub fn run(target: AddTarget) -> Result<()> {
    match target {
        AddTarget::App { target } => add_app(&target),
        AddTarget::Module { name } => add_module(&name),
        AddTarget::Firmware { target } => add_firmware(&target),
        AddTarget::Component { name, framework } => {
            crate::commands::component::run(&name, &framework)
        }
        AddTarget::Eda => install_capability("eda"),
        AddTarget::I18n => install_capability("i18n"),
        AddTarget::Brand => install_capability("brand"),
        AddTarget::Deploy => install_capability("deploy"),
        AddTarget::Identity => install_capability("identity"),
        AddTarget::Legal => install_capability("legal"),
        AddTarget::Adapters => install_capability("adapters"),
        AddTarget::Migrations => install_capability("migrations"),
        AddTarget::Capability { id, from } => match from {
            Some(spec) => install_external(&id, &spec),
            None => install_capability(&id),
        },
    }
}

/// Install a capability resolved from outside the binary.
///
/// The only difference from a built-in is where the bytes came from: the
/// derivation, the conformance check and the installer are the same code. That
/// is the whole claim of this feature, so it is worth stating where it is true.
fn install_external(id: &str, spec: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;

    let spec = capability::source::Spec::parse(spec)?;
    let cache = root.join(".fiducial/capabilities");
    let cap = capability::source::resolve(&spec, id, &cache)?;

    // A third party's capability is held to the rules a first party's is. It
    // arrives as files written into someone's repository, so "it came from
    // outside" is a reason for more checking, not less.
    let problems = capability::check_capability(&cap);
    if !problems.is_empty() {
        for p in &problems {
            eprintln!("  ✗ {p}");
        }
        anyhow::bail!(
            "`{id}` does not conform, so it was not installed.\n\
             These are the same checks every built-in capability passes."
        );
    }

    let cfg = Config::load(&root.join(CONFIG_FILE))?;
    capability::install(&cap, &root, &cfg.product.name)?;
    println!("  ✓ source: {}", cap.source.label());
    Ok(())
}

fn add_app(target: &str) -> Result<()> {
    let cap_id = match target {
        "next" => "web-next",
        "svelte" => "web-svelte",
        "tauri" => "tauri",
        "worker" => "worker-cloudflare",
        "mobile" => {
            println!(
                "✦ fid add app {target}\n\n\
                 The `{target}` target is not available yet.\n\
                 See SHIPPED.md: https://github.com/AleksaZCodes/fiducial/blob/main/SHIPPED.md"
            );
            return Ok(());
        }
        other => bail!(
            "unknown app target `{other}`.\n\
             Supported: next, svelte, tauri, worker, mobile.\n\
             Run `fid add app --help` for details."
        ),
    };
    install_capability(cap_id)
}

fn add_firmware(target: &str) -> Result<()> {
    let cap_id = match target {
        "rp2040" => "firmware-rp2040",
        "stm32" => "firmware-stm32",
        "rp2350" => {
            println!(
                "✦ fid add firmware rp2350\n\n\
                 The `rp2350` firmware target is coming in a future phase.\n\
                 It reuses the same Embassy setup as rp2040 with the RP2350 HAL.\n\
                 Track progress: https://github.com/AleksaZCodes/fiducial"
            );
            return Ok(());
        }
        "nrf52" => {
            println!(
                "✦ fid add firmware nrf52\n\n\
                 The `nrf52` firmware target is coming in a future phase.\n\
                 Track progress: https://github.com/AleksaZCodes/fiducial"
            );
            return Ok(());
        }
        other => bail!(
            "unknown firmware target `{other}`.\n\
             Supported: rp2040, rp2350, stm32, nrf52.\n\
             Run `fid add firmware --help` for details."
        ),
    };
    install_capability(cap_id)
}

fn add_module(name: &str) -> Result<()> {
    println!(
        "✦ fid add module {name}\n\n\
         Module scaffolding is not available yet.\n\
         See SHIPPED.md: https://github.com/AleksaZCodes/fiducial/blob/main/SHIPPED.md"
    );
    Ok(())
}

// ── Capability installer ──────────────────────────────────────────────────────

/// Install a capability by id into the current product.
pub fn install_capability(cap_id: &str) -> Result<()> {
    let cwd = env::current_dir().context("getting current directory")?;
    let root = Config::find_root(&cwd)?;
    let cfg = Config::load(&root.join(CONFIG_FILE))?;

    // Already installed?
    if cfg.capabilities.enabled.contains(&cap_id.to_string()) {
        println!(
            "  ✓ `{cap_id}` is already installed in `{}`.",
            cfg.product.name
        );
        println!("  Run `fid capability check` to verify it is in good shape.");
        return Ok(());
    }

    // Look up the capability definition.
    let def = capability::find(cap_id).ok_or_else(|| {
        anyhow::anyhow!(
            "capability `{cap_id}` not found in the built-in registry.\n\
             Run `fid capability list --all` to see available capabilities.\n\
             Only built-in capabilities are supported; third-party ones are not."
        )
    })?;

    capability::install(def, &root, &cfg.product.name)
}
