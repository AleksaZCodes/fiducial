use anyhow::Result;
use clap::{Parser, Subcommand};

mod capability;
mod commands;
mod config;
mod guard;
mod lock;
mod migration;
mod templates;

// ── Top-level CLI ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "fid",
    about = "Fiducial — one declaration, many derivations",
    long_about = "\
Fiducial is a platform for building cross-domain products — web, firmware,
desktop, PCB, mechanical, simulation — from a single declared source of truth.

Every fact is declared once. Every artifact is derived from it. Nothing correct
is solved twice.

Use `fid new` to scaffold a product, `fid add` to grow it, `fid derive` to run
its pipelines, and `fid doctor` to verify it stays in sync with the platform.",
    after_long_help = "\
QUICK START
  fid new my-product          scaffold a new product repository
  cd my-product
  fid add app next            add a Next.js web app
  fid add firmware rp2040     add RP2040 Embassy firmware
  fid doctor                  verify the product is in order
  fid derive                  run all pipelines
  fid derive --check          fail CI if any artifact is stale

GUARD
  Products scaffolded with `fid new` configure a Claude Code PreToolUse hook
  that calls `fid guard-check` before every Bash invocation. The guard is
  shell-aware: it tokenizes the command and matches only in command position
  (argv[0]), so `echo \"use npm\"` never triggers the package-manager rule.

DOCS
  https://github.com/AleksaZCodes/fiducial",
    version,
    propagate_version = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scaffold a new product repository
    #[command(
        long_about = "\
Scaffold a new product repository named <NAME>.

Creates the directory, writes all platform template files into it, records each
one in `fiducial.lock` (so `fid upgrade` can 3-way-merge upstream changes while
preserving local edits), and runs `git init`.

Creates NO cloud resources. Everything is local until you `git push`.",
        after_long_help = "\
TEMPLATE FILES WRITTEN
  fiducial.toml          product config — name, spine flag, capabilities, guard rules
  fiducial.lock          template version tracking (do not edit by hand)
  MISSION.md             one paragraph: what this product is for and who it serves
  AGENTS.md              agent context loaded by Claude Code every session
  .claude/settings.json  PreToolUse guard hook configuration
  .gitignore             standard ignores for Rust, Node, Fiducial artifacts
  README.md              getting-started instructions

NAME RULES
  Lowercase letters, digits, and hyphens only. No leading or trailing hyphens.
  Becomes the product identifier in fiducial.toml, Cargo workspace names, and
  the @scope/package prefix on the JS registry.

EXAMPLES
  fid new ring-of-pursuit     web + realtime + edge product
  fid new lora-walkie         PCB + firmware + Web Bluetooth + enclosure
  fid new my-saas             pure-TypeScript SaaS (no Rust spine needed)"
    )]
    New {
        /// Product name: lowercase letters, digits, and hyphens (e.g. `my-product`)
        #[arg(
            value_name = "NAME",
            long_help = "\
The product identifier. Used as:
  - The directory name created by `fid new`
  - The `product.name` field in fiducial.toml
  - The prefix for Cargo crate names if the Rust spine is enabled
  - The npm package scope prefix (@<name>/*) if JS packages are added

Rules: lowercase ASCII letters, digits, and hyphens. No leading or trailing
hyphens. Examples: `my-product`, `ring-of-pursuit`, `lora-walkie`."
        )]
        name: String,
    },

    /// Add an app, module, or firmware target to this product
    #[command(
        long_about = "\
Add a capability, app, or bounded-context module to the current product.

With a SUBCOMMAND (app / module / firmware), scaffold a specific kind of target.
With a CAPABILITY name, install a platform capability: write its templates, add
its guard rules to fiducial.toml, and activate its Claude Code skill.",
        after_long_help = "\
EXAMPLES
  fid add app next            add a Next.js web app
  fid add app svelte          add a SvelteKit web app
  fid add app tauri           add a Tauri desktop + mobile app
  fid add app worker          add a Cloudflare Worker
  fid add firmware rp2040     add RP2040 Embassy firmware
  fid add firmware rp2350     add RP2350 Embassy firmware
  fid add module payments     add a bounded-context Rust + TS module
  fid add web-next            install the web-next capability (alias for `fid add app next`)
  fid add firmware-rp2040     install the firmware-rp2040 capability directly"
    )]
    Add {
        #[command(subcommand)]
        target: commands::add::AddTarget,
    },

    /// Manage capabilities installed in this product
    #[command(
        long_about = "\
List, inspect, scaffold, or validate Fiducial capabilities.

A capability is a named extension to the platform. It may contribute fact
schemas, pipelines, crates/packages, template files, guard rules, migrations,
and a SKILL.md that agents read on session start. Only SKILL.md is required —
the minimum viable capability is a single file.

Capabilities are versioned alongside platform packages. `fid upgrade` propagates
capability updates to every product that has installed them.",
        after_long_help = "\
EXAMPLES
  fid capability list                     show all installed capabilities and versions
  fid capability new my-sensor            scaffold a new first-party capability
  fid capability check                    validate all installed capabilities
  fid capability check --capability eda   validate one capability"
    )]
    Capability {
        #[command(subcommand)]
        action: commands::capability::CapabilityAction,
    },

    /// Run declared pipelines (`--check` fails CI on stale artifacts)
    #[command(
        long_about = "\
Run every pipeline declared in this product's capability graph.

Each pipeline is a pure transformation: (facts, artifacts) → artifacts. Running
`fid derive` executes them in dependency order. Passing `--check` validates
freshness without writing — use this in CI to fail on any stale artifact or
violated assertion.",
        after_long_help = "\
EXAMPLES
  fid derive                  run all pipelines, write outputs
  fid derive --check          fail if any artifact is stale (CI mode)
  fid derive --pipeline eda   run only the EDA pipeline and its dependencies

STATUS
  Not yet implemented (Phase 4). Run `cargo build` / `wasm-pack build` directly
  until `fid derive` is wired up."
    )]
    Derive {
        /// Fail if any artifact is stale or any assertion is violated; write nothing
        #[arg(long, default_value_t = false)]
        check: bool,
        /// Run only a specific pipeline (and its dependencies)
        #[arg(long, value_name = "NAME")]
        pipeline: Option<String>,
    },

    /// Pull upstream template and package updates into this product
    #[command(
        long_about = "\
Propagate platform updates into this product.

`fid upgrade` applies changes in three layers:

1. SEMVER — bumps package and crate versions.
2. CODEMODS — applies migrations for API renames and reshapes (ts-morph for
   TypeScript, AST rewrites for Rust). Required only when a change affects more
   than one product.
3. TEMPLATE MERGE — 3-way-merges upstream template changes against local edits.
   fiducial.lock records the version each file came from; upgrade diffs upstream
   between that version and the new one and merges against local, surfacing
   conflicts exactly like `git merge`.

The model is identical to `rails app:update`.",
        after_long_help = "\
EXAMPLES
  fid upgrade                 upgrade everything to the latest platform release
  fid upgrade --dry-run       show what would change without writing
  fid upgrade --portfolio     upgrade all products in the portfolio manifest

STATUS
  Not yet implemented (Phase 4)."
    )]
    Upgrade {
        /// Show what would change without writing anything
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Upgrade all products listed in the portfolio manifest
        #[arg(long, default_value_t = false)]
        portfolio: bool,
    },

    /// Emit the facts → pipelines → artifacts dependency graph
    #[command(
        long_about = "\
Emit the product's dependency graph as structured data.

The graph connects declared facts to the pipelines that consume them and the
artifacts they produce. It is queryable by humans, agents, and the workbench.
Agents use it to understand what is stale, what a change affects, and what must
be re-derived.",
        after_long_help = "\
OUTPUT FORMATS
  --format dot    Graphviz DOT (pipe to `dot -Tsvg` for a diagram)
  --format json   Machine-readable JSON
  --format text   Human-readable tree (default)

EXAMPLES
  fid graph
  fid graph --format dot | dot -Tsvg -o graph.svg
  fid graph --format json | jq '.nodes[] | select(.kind == \"artifact\")'

STATUS
  Not yet implemented (Phase 4)."
    )]
    Graph {
        /// Output format: text | dot | json
        #[arg(long, default_value = "text", value_name = "FORMAT")]
        format: String,
    },

    /// Check for drift: outdated deps, stale templates, un-applied migrations
    #[command(
        long_about = "\
Check the current product for drift from the platform baseline.

Reads `fiducial.toml` and `fiducial.lock`, validates that every tracked template
file matches its recorded SHA-256 hash, and reports any issues. Exits non-zero
if any issue is found — safe to run in CI as a pre-flight check.

Phase 3 checks (now):
  - fiducial.toml exists and parses correctly
  - fiducial.lock exists and parses correctly
  - Every template file recorded in the lock is present and unmodified

Phase 4+ checks (coming):
  - Available migrations not yet applied
  - Installed capability versions behind the platform
  - Stale derived artifacts (runs `fid derive --check` internally)",
        after_long_help = "\
EXAMPLES
  fid doctor                  check the product in the current directory
  fid doctor --portfolio      check all products in the portfolio manifest (Phase 4+)

EXIT CODES
  0   Clean — no drift detected
  1   One or more issues found (details printed to stdout)"
    )]
    Doctor,

    /// Shell-aware guard hook — reads PreToolUse JSON from stdin, exits non-zero to block
    ///
    /// This subcommand is called by Claude Code's PreToolUse hook mechanism,
    /// not by humans directly. It reads the tool-call JSON from stdin, tokenizes
    /// the Bash command with a shell-aware parser, and checks each command-position
    /// token against the guard rules declared in fiducial.toml.
    ///
    /// The key invariant: a rule fires only when the forbidden token is in argv[0]
    /// position. `echo "curl example.com"` never triggers the curl rule because
    /// "curl" is a quoted argument, not a command.
    #[command(
        hide = true,
        after_long_help = "\
PROTOCOL
  Input (stdin):  {\"tool_name\": \"Bash\", \"tool_input\": {\"command\": \"...\"}}
  Output (stdout): block reason (only when exiting non-zero)
  Exit 0: allow the tool call
  Exit 1: block the tool call (reason printed to stdout for Claude Code to display)"
    )]
    GuardCheck,
}

// ── Entry point ──────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name } => commands::new::run(&name),
        Commands::Add { target } => commands::add::run(target),
        Commands::Capability { action } => commands::capability::run(action),
        Commands::Derive { check, pipeline } => commands::derive::run(check, pipeline),
        Commands::Upgrade { dry_run, portfolio } => commands::upgrade::run(dry_run, portfolio),
        Commands::Graph { format } => commands::graph::run(&format),
        Commands::Doctor => commands::doctor::run(),
        Commands::GuardCheck => guard::check_from_stdin(),
    }
}
