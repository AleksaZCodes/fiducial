use anyhow::Result;
use clap::{Parser, Subcommand};

mod adapter;
mod brand;
mod capability;
mod commands;
mod config;
mod context;
mod design;
mod guard;
mod i18n;
mod legal;
mod lock;
mod migration;
mod pipeline;
mod prose;
mod schema;
mod security;
mod templates;
mod thesis;

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
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Repository configuration on the host, as opposed to in the working tree.
#[derive(Subcommand)]
enum RepoCmd {
    /// Protect the default branch with what this product already declares
    #[command(long_about = "\
`no-direct-main-push` is a local hook. It fires on `git push` from a machine
that has the hook installed — and on no other machine, and on nothing a token
or a CI job does. A product declaring it believes main is protected while
GitHub happily accepts a push to it.

That is the same failure this platform refuses to shrug at one layer in: a
guard rule listed with no implementation is a finding, because a product
listing it believes it is guarded and is not. The remote is the other half of
that sentence.

Nothing applied here is a preference. Required checks come from the job names
in .github/workflows/ — a list kept by hand goes stale on the next renamed job,
and a required check nothing produces blocks every PR forever. Pull requests,
force pushes and deletions come from the guard rule itself.

Needs `gh`, logged in, with admin on the repository.

EXAMPLES
  fid repo protect            show what would be applied, change nothing
  fid repo protect --apply    write it to GitHub")]
    Protect {
        /// Write the policy to GitHub instead of printing it
        #[arg(long)]
        apply: bool,
        /// Do not require this job, by name. Repeatable.
        ///
        /// The escape hatch for a job a workflow cannot describe. Prefer
        /// `continue-on-error: true` on the job itself — that is a declaration
        /// the repository keeps, and this is a flag someone has to remember.
        #[arg(long = "except", value_name = "JOB")]
        except: Vec<String>,
    },
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

        /// Locales this product ships, comma-separated; `none` to opt out
        #[arg(
            long,
            value_name = "LIST",
            default_value = "sr,en",
            long_help = "Locales the product ships, comma-separated — for example `en,fr,de`.

The product is created localized. Principle 1c says a user-visible string is a
fact with one derivation per locale, and monolingual is a state you pass through
before the first commit, not one you ship: added later, localization is a
refactoring pass over strings that have already been missed.

A catalog is written for every locale named here. Where the platform ships one
(sr, en) it is used; otherwise the locale starts as a copy of the default, which
`fid derive` then reports as untranslated — visible work rather than a silent
gap.

Pass `--locales none` for a product that genuinely has no user-visible text,
such as a CLI or a firmware image."
        )]
        locales: Option<String>,

        /// Locale a reader falls back to; must be one of --locales
        #[arg(
            long,
            value_name = "LOCALE",
            long_help = "The locale a reader falls back to when lookup fails.

Defaults to `sr`. It is NOT the first entry of --locales: which language a
product falls back to is a decision, and inferring it from list order is the
kind of implicit fact this platform exists to delete. Name a locale set other
than the default and this must be named too."
        )]
        default_locale: Option<String>,
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

EXECUTORS
  cargo-test     run `cargo test` (used by the built-in types pipeline)
  shell          run an external tool, e.g. kicad-cli
  fid-validate   validate an artifact against a schema, in-process
  fid-mesh       generate case geometry from a board outline, in-process

SEE ALSO
  fid graph   what each pipeline produces
  fid dash    the same, alongside what is currently stale"
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
  Semver bumps, codemods, and template merging all work. `--portfolio`
  fan-out across repositories does not; run `fid upgrade` in each product."
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

SEE ALSO
  fid dash    the same graph alongside roadmap, decisions, CI and freshness"
    )]
    Graph {
        /// Output format: text | dot | json
        #[arg(long, default_value = "text", value_name = "FORMAT")]
        format: String,
    },

    /// Platform version management and version-skew enforcement
    #[command(
        long_about = "\
Manage the platform wire-protocol version and verify the committed compatibility
matrix stays in sync with the compiled constant.

SUBCOMMANDS
  fid release status                      show current versions and matrix
  fid release check                       fail if matrix ≠ WIRE_VERSION (CI gate)
  fid release protocol --bump breaking    bump wire version, drop old artifacts
  fid release protocol --bump compatible  bump wire version, keep old artifacts

The wire protocol version (`WIRE_VERSION` in `crates/fiducial-protocol/src/lib.rs`)
is embedded in every compiled artifact. When two endpoints connect they exchange
this version; `assert_compatible` rejects anything below the declared minimum.

The committed `docs/compat/matrix.toml` records the current version and the
minimum accepted version. `fid release check` enforces that the file and the
constant agree — a PR that bumps one without the other fails CI.",
        after_long_help = "\
EXAMPLES
  fid release status
  fid release check
  fid release protocol --bump breaking --note \"Removed legacy handshake field\"
  fid release protocol --bump compatible --note \"Added optional capabilities byte\"

SEE ALSO
  docs/compat/matrix.toml   the committed compatibility policy
  fid dash                  freshness and pipeline state at a glance"
    )]
    Release {
        #[command(subcommand)]
        action: commands::release::ReleaseAction,
    },

    /// The thesis — the one claim this product is built to test
    #[command(
        long_about = "\
Read, declare and sharpen this product's thesis.

A thesis is the single claim a product exists to test: the sentence you would
sell with, that a reasonable person could disagree with, and that some future
observation could prove wrong. It is declared in `thesis.toml`, once, with its
parts separable — the arc you would pitch, the test that makes it falsifiable,
and the evidence you actually have.

It is a Fact with a Decision's lifecycle. `thesis.toml` is append-only: a
sharper claim supersedes the old one and says why, and the old wording is never
edited away, because how the thinking matured is most of what a reader wants.

One rough line is a valid thesis. `claim` is the only required field; every
other part is optional and answerable months later. Nothing here gates a build,
and `fid derive` is byte-identical whether a thesis is complete or a stub.

  fid thesis                      what the product currently claims
  fid thesis log                  how that claim got here
  fid thesis set \"<claim>\"        declare or sharpen it

What is derived from it — PITCH.md, a TypeScript module the app imports — comes
from the `fid-thesis` pipeline. `fid advise` argues with the claim itself.",
        after_long_help = "\
EXAMPLES
  fid thesis set \"No unverified alert ever reaches a responder.\"
  fid thesis set \"<sharper>\" --because \"Cheap was never the objection.\"
  fid thesis log

SEE ALSO
  thesis.toml    the declaration
  PITCH.md       the arc, assembled by `fid derive`
  fid advise     is it falsifiable? could anyone disagree?"
    )]
    Thesis {
        #[command(subcommand)]
        action: Option<commands::thesis::ThesisAction>,

        /// Emit the current thesis, its gaps and its history as JSON
        ///
        /// The machine-readable view other tools read rather than re-parsing
        /// `thesis.toml`. `fid advise` uses it, so there is one parser for the
        /// declaration and it is the one with the tests behind it.
        #[arg(long, global = true)]
        json: bool,
    },

    /// The workbench — one read-only view of roadmap, decisions, CI, graph, freshness
    #[command(
        long_about = "\
Render the workbench: everything about this product's state in one view.

The repository is the database; this is a view over it. `fid dash` owns no
store, caches nothing, and writes nothing — every number is recomputed from
files already in the repo, so it cannot go stale the way a synced dashboard
can, and there is nothing to invalidate when the repo changes.

It makes no network calls. CI is reported from the workflow files the repo
declares, not from a live API: a view that needs a token and a connection to
render is a view that stops working on a plane, and a cached answer would be a
private store by another name.

Absence is reported, not treated as an error. A product with no roadmap, no
decisions, or no pipelines gets a section saying so.

SECTIONS
  product     name, version, capabilities, guard rules
  git         branch, head, working tree, upstream divergence
  roadmap     progress counted from ROADMAP.md or SHIPPED.md
  decisions   dated records in docs/specs (or docs/decisions, docs/adr)
  ci          declared workflows, their triggers, whether any checks freshness
  graph       pipelines and the artifacts they produce
  freshness   artifacts re-hashed against fiducial.lock, plus template drift",
        after_long_help = "\
EXAMPLES
  fid dash                        the whole view
  fid dash --section freshness    just what is stale
  fid dash --json                 same facts for agents and workbench v1
  fid dash --json | jq '.freshness.problems'

EXIT CODES
  0   Rendered. Dash reports problems; it does not fail on them —
      use `fid derive --check` or `fid doctor` for that."
    )]
    Dash {
        /// Emit JSON instead of text — the same facts, for agents and tooling
        #[arg(long)]
        json: bool,
        /// Render only one section
        #[arg(long, value_name = "NAME")]
        section: Option<String>,
        /// Aggregate all products in fiducial.portfolio (workbench v1)
        #[arg(long)]
        portfolio: bool,
    },

    /// Survey an existing codebase for reusable logic, art, UI and principles
    #[command(
        long_about = "\
Survey a repository or folder for work worth reusing, and stage it for review.

You have built something before, and it contains things you should not build
again: the business rules you got right, the theme you spent a week tuning, the
components, the conventions you arrived at the hard way. Rebuilding those in the
next product is the most expensive habit in independent software work.

This command does the mechanical half. It walks the source, classifies every
file as logic / ui / theme / art / principle / ops / contract / test, detects the
donor's stack, and writes two things into `harvest/<name>/`:

  harvest.toml   machine-readable inventory — what an agent reads
  SURVEY.md      the human survey, ordered by value per unit of risk

It stages readable copies under `harvest/<name>/assets/`.

It does NOT decide what is worth keeping, and it does NOT touch your source
tree. Whether a function is business logic or incidental framing, whether a
component generalizes or encodes one product's assumptions — none of that is a
heuristic. Run `/fiducial:harvest` afterwards for that half.

Nothing is wired in. `harvest/` is a staging area; the donor stays reference
material until you decide what to lift and how to generalize it.",
        after_long_help = "\
EXAMPLES:
    fid harvest ../old-web-app
    fid harvest ~/dev/ring-of-pursuit --name rop
    fid harvest ../donor --json | jq '.summary'

AFTERWARDS:
    /fiducial:harvest <name>    work through the extraction with an agent"
    )]
    Harvest {
        /// Path to the repository or folder to survey
        #[arg(value_name = "PATH")]
        path: String,
        /// Name for the staging directory (defaults to the source folder name)
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        /// Where to stage (default: harvest/)
        #[arg(long, value_name = "DIR")]
        into: Option<String>,
        /// Emit JSON instead of text — the same facts, for agents and tooling
        #[arg(long)]
        json: bool,
    },

    /// Regenerate the derivable parts of AGENTS.md / CLAUDE.md
    #[command(
        long_about = "\
Regenerate the blocks of agent context that are facts about this repository.

The layout tree, the command list, the capability list and where the skills live
are all derivable from the things that decide them — the workspace manifests,
this binary's own command definitions, the capability registry, the commands
directory. Written by hand they go stale, and a test that catches that
afterwards is detection, not sync.

Blocks are marked with HTML comments, so they render as nothing and a file
without them is left completely alone:

  <!-- fid:begin layout -->
  <!-- fid:end layout -->

Judgment stays hand-written. Nothing can derive what not to do.",
        after_long_help = "\
BLOCKS
  layout         crates/ and packages/, with each member's own description
  commands       every `fid` command, from this binary's definition
  capabilities   every built-in capability and what it contributes
  skills         where the instructions an agent can load live

EXAMPLES
  fid context            regenerate every marked block
  fid context --check    fail when a block is out of date (the CI gate)"
    )]
    Context {
        /// Fail instead of rewriting, for CI
        #[arg(long)]
        check: bool,
    },

    /// Is the published design gallery the system this product declares?
    #[command(long_about = "\
Check the design gallery before it is published.

This does not upload — the upload is `/design-sync`, which runs in an agent
holding your claude.ai authorization. What this owns is the half that fails
silently:

  • The gallery is rebuilt by a `node` script, separately from `fid derive`.
    Run the derive without the rebuild and the published swatches describe a
    palette the product no longer has, while every other gate stays green.

  • The Design System pane indexes each preview by its first-line
    `<!-- @dsCard group=\"…\" -->` marker. A page without one uploads fine and
    then is simply not there.

EXAMPLES
  fid design           list the cards and report both checks
  fid design --check   fail when either is wrong (the CI gate)
  fid design --plan    print the write set for /design-sync")]
    Design {
        /// Fail instead of reporting, for CI
        #[arg(long)]
        check: bool,
        /// Print the `/design-sync` write set as JSON
        #[arg(long, conflicts_with = "check")]
        plan: bool,
    },

    /// Make the remote enforce what `[guard]` already claims
    #[command(subcommand)]
    Repo(RepoCmd),

    /// Advisory review of the rules code cannot check — asks, never gates
    #[command(
        args_conflicts_with_subcommands = true,
        long_about = "\
Ask a decision model about the things this platform's deterministic checks
cannot reach, and print what it says.

WHAT IT ASKS, AND WHY A MODEL AT ALL
  Every question here is one code cannot answer. A regex can see that two
  constants both hold 1.6; it cannot see that `board_thickness` and
  `pcb_height` are one fact declared twice — which is the precise violation
  principle 1 exists to prevent. The checks:

    a fact may be declared twice        semantic, not textual
    logic sits above the layer it could reach   requires knowing what it does
    a physical quantity has no unit or tolerance   code narrows, model judges
    an abstraction has no escape hatch  principle 3, a judgment
    a comment explains what, not why    a judgment about prose
    the diff does more than was asked   scope, a judgment
    the change is left half-finished    a judgment

WHAT IT DELIBERATELY DOES NOT ASK
  Anything already decided exactly: whether an artifact was hand-edited
  (`fid derive --check`), whether a translation is missing (the i18n pipeline),
  contrast ratios (`fid-design`), a cost ceiling (arithmetic), a push to main
  (the guard). Spending a probabilistic answer where a certain one exists is a
  regression dressed as a feature.

IT CANNOT BREAK ANYTHING
  Exits 0 even with findings, and when the key is missing, the network is down,
  or the model is overloaded. Pass --strict to opt into a non-zero exit for a
  hook of your own — never wire it into a gate that must not flake.

SEEING WHAT LEAVES THE MACHINE
  This sends your diff to a third-party API. `--dry-run` resolves no key, sends
  nothing, and prints the exact request body instead.

EXAMPLES
  fid advise                                  review uncommitted changes
  fid advise --task \"fix the retry bug\"        judge scope against the ask
  fid advise --base main                      review the whole branch
  fid advise --facts                          duplicate-check new declarations
  fid advise --dry-run                        show what would be sent
  fid advise key set                          store the key that unlocks this
  fid advise status                           is it available, and what runs"
    )]
    Advise {
        /// Manage the key, or report availability
        #[command(subcommand)]
        cmd: Option<commands::advise::AdviseCmd>,
        /// Check new declarations for semantic duplicates instead of reviewing a diff
        #[arg(long)]
        facts: bool,
        /// Argue with this product's thesis instead of reviewing a diff
        ///
        /// Is the claim falsifiable? Could a reasonable person disagree? Does
        /// the product's own copy sell something else, or claim more than the
        /// evidence supports? Advisory, like everything else here — it cannot
        /// change a file and it cannot fail a build.
        #[arg(long)]
        thesis: bool,
        /// Review everything since this ref rather than uncommitted changes
        #[arg(long, value_name = "REF")]
        base: Option<String>,
        /// What was actually asked for, so scope can be judged against it
        #[arg(long, value_name = "TEXT")]
        task: Option<String>,
        /// Print the request body and send nothing
        #[arg(long)]
        dry_run: bool,
        /// Machine-readable output
        #[arg(long)]
        json: bool,
        /// Exit non-zero on a firm finding. Never use in a gate that must not flake.
        #[arg(long)]
        strict: bool,
    },

    /// Documentation freshness, including prose nothing can generate
    #[command(long_about = "\
Check that documentation still matches what it describes.

`fid context` keeps GENERATED blocks current. This keeps HAND-WRITTEN ones
honest, which nothing did before: a paragraph goes stale silently, and every
other gate stays green while it does. AGENTS.md said every adapter contract
implemented only `none` for six vendors after that stopped being true.

Prose is not generated — a paragraph explaining why a contract is shaped a
certain way cannot be derived from the contract, or it would carry nothing the
code does not. What is derived is the OBLIGATION TO REVISIT IT.

A block declares what it describes:

  <!-- fid:describes crates/fiducial-cli/src/adapter.rs#CONTRACTS -->
  Adapters name contracts, not vendors…
  <!-- fid:end-describes -->

When that source changes and the prose was not revisited, --check fails and
names both. Narrow the watch with #Symbol: a whole-file hash fires on every
unrelated edit, and a gate that cries wolf trains you to accept without
reading.

EXAMPLES
  fid docs             report which blocks need re-reading
  fid docs --check     fail when one does (the CI gate)
  fid docs --accept    record that you have read them against their sources")]
    Docs {
        /// Fail instead of reporting, for CI
        #[arg(long)]
        check: bool,
        /// Record every watched source as read
        #[arg(long, conflicts_with = "check")]
        accept: bool,
    },

    /// Check for drift: outdated deps, stale templates, un-applied migrations
    #[command(
        long_about = "\
Check the current product for drift from the platform baseline.

Reads `fiducial.toml` and `fiducial.lock`, validates that every tracked template
file matches its recorded SHA-256 hash, and reports any issues. Exits non-zero
if any issue is found — safe to run in CI as a pre-flight check.

Checks:
  - fiducial.toml exists and parses correctly
  - fiducial.lock exists and parses correctly
  - Every template file recorded in the lock is present and unmodified
  - Template files behind the current platform version
  - Codemod migrations recorded as pending

Not checked here: derived-artifact freshness. Run `fid derive --check` for
that, or `fid dash` to see it alongside everything else.",
        after_long_help = "\
EXAMPLES
  fid doctor                  check the product in the current directory
  fid doctor --portfolio      check every product in the portfolio manifest (not available yet)

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
        Commands::New {
            name,
            locales,
            default_locale,
        } => commands::new::run(&name, locales.as_deref(), default_locale.as_deref()),
        Commands::Add { target } => commands::add::run(target),
        Commands::Capability { action } => commands::capability::run(action),
        Commands::Derive { check, pipeline } => commands::derive::run(check, pipeline),
        Commands::Upgrade { dry_run, portfolio } => commands::upgrade::run(dry_run, portfolio),
        Commands::Graph { format } => commands::graph::run(&format),
        Commands::Release { action } => commands::release::run(action),
        Commands::Thesis { action, json } => commands::thesis::run(action, json),
        Commands::Dash {
            json,
            section,
            portfolio,
        } => commands::dash::run(json, section, portfolio),
        Commands::Harvest {
            path,
            name,
            into,
            json,
        } => commands::harvest::run(&path, name, into, json),
        Commands::Context { check } => commands::context::run(check),
        Commands::Design { check, plan } => commands::design::run(check, plan),
        Commands::Repo(RepoCmd::Protect { apply, except }) => commands::repo::run(apply, &except),
        Commands::Advise { cmd: Some(cmd), .. } => commands::advise::run(cmd),
        Commands::Advise {
            cmd: None,
            facts,
            thesis,
            base,
            task,
            dry_run,
            json,
            strict,
        } => {
            // `--thesis` wins over `--facts` when both are given rather than
            // silently reviewing declarations: the thesis is the narrower,
            // more deliberate request, and picking it makes the mistake
            // visible in the output instead of producing a plausible report
            // about something else.
            let mode = if thesis {
                "thesis"
            } else if facts {
                "facts"
            } else {
                "diff"
            };
            let mut args: Vec<String> = vec![mode.to_string()];
            if let Some(base) = &base {
                args.push("--base".into());
                args.push(base.clone());
            }
            if let Some(task) = &task {
                args.push("--task".into());
                args.push(task.clone());
            }
            if dry_run {
                args.push("--dry-run".into());
            }
            if json {
                args.push("--json".into());
            }
            if strict {
                args.push("--strict".into());
            }
            let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
            commands::advise::delegate(&borrowed)
        }
        Commands::Docs { check, accept } => commands::docs::run(check, accept),
        Commands::Doctor => commands::doctor::run(),
        Commands::GuardCheck => guard::check_from_stdin(),
    }
}
