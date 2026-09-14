//! `fid harvest <path>` — survey an existing codebase for reusable assets.
//!
//! # What this command is for
//!
//! You have built something before — a web app, a monorepo, a prototype — and it
//! contains work you do not want to do again: the business rules you got right,
//! the theme you spent a week tuning, the components, the copy, the conventions
//! you arrived at the hard way. Rebuilding those in the next product is the
//! single most expensive habit in independent software work, and
//! [`MISSION.md`](../../../MISSION.md) names it as an anti-goal: *we do not
//! rebuild what already works.*
//!
//! # What this command does, and deliberately does not do
//!
//! It **surveys**. It walks a source tree, classifies what it finds, and writes
//! a machine-readable inventory plus a human-readable survey into a staging
//! directory. It copies reference material there for the agent to read.
//!
//! It does **not** decide what is worth keeping, and it does not touch your
//! product's source tree. That separation is the whole design:
//!
//! - **Mechanical work is the command's.** Walking, hashing, classifying,
//!   measuring, detecting frameworks — that is deterministic, boring and fast,
//!   and an agent doing it by hand burns context on `ls` output.
//! - **Judgment is the agent's.** Whether a function is *business logic* or
//!   incidental framing, whether a component generalizes or encodes one
//!   product's assumptions, whether a convention is a principle or a habit —
//!   none of that is a heuristic. `SKILL.md` drives that half.
//!
//! # Why staging, rather than importing
//!
//! Everything lands in `harvest/<slug>/`, which is **not** your source tree.
//! Nothing is overwritten, nothing is wired in, nothing compiles. The donor
//! repository is reference material until a human or agent has read it and
//! decided what to lift and how to generalize it.
//!
//! A tool that imported directly would produce the thing this platform exists to
//! prevent: a second copy of somebody's assumptions, pasted into a new product,
//! now needing to be kept in sync with a repository nobody looks at again.

use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Directories never worth walking. These hold build output, dependencies and
/// version-control internals — megabytes that classify as nothing.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".cache",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    "coverage",
    ".pytest_cache",
    ".gradle",
    "Pods",
    "DerivedData",
    ".terraform",
    "harvest",
    // Found by running this command on a real monorepo: these hold vendored
    // dependency caches and tool scratch space, and their contents outranked
    // every genuine source file in the survey. A 13,000-line VitePress dep
    // chunk is not the donor's most valuable business logic.
    ".vitepress",
    ".wrangler",
    ".temp",
    ".tmp",
    "tmp",
    ".output",
    ".vercel",
    ".astro",
    "storybook-static",
    "playwright-report",
    "test-results",
    ".nyc_output",
    "cache",
];

/// Files larger than this are inventoried but never staged. A 40 MB PSD is a
/// real asset and worth recording; copying it into the staging area is not.
const MAX_STAGE_BYTES: u64 = 512 * 1024;

/// What a file is, for the purpose of reuse.
///
/// These categories are the ones the request actually named — business logic,
/// art, UI/UX, and principles — plus the two that always come with them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// Pure rules: no framework imports, no I/O. The most portable thing in any
    /// codebase and the most expensive to rewrite, because the value is in the
    /// edge cases someone already found.
    Logic,
    /// Components and views.
    Ui,
    /// Design tokens, CSS custom properties, Tailwind config, theme files.
    Theme,
    /// Images, icons, fonts, audio, 3D.
    Art,
    /// Prose that encodes judgment: decision records, conventions, READMEs.
    Principle,
    /// Scripts, workflows, deployment, infrastructure.
    Ops,
    /// Schemas, migrations, type declarations shared across boundaries.
    Contract,
    /// Tests — kept because they are the executable statement of what the logic
    /// is supposed to do, and are often the fastest way to understand it.
    Test,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Logic => "logic",
            Kind::Ui => "ui",
            Kind::Theme => "theme",
            Kind::Art => "art",
            Kind::Principle => "principle",
            Kind::Ops => "ops",
            Kind::Contract => "contract",
            Kind::Test => "test",
        }
    }

    /// One line on why this category is worth a human's attention.
    fn why(self) -> &'static str {
        match self {
            Kind::Logic => {
                "Pure rules — the most portable and most expensive-to-rewrite thing here. \
                 Push it down to L0 (`no_std` Rust) if it is genuinely domain logic."
            }
            Kind::Ui => {
                "Components. Generalize the shape, drop the product's nouns, and land it \
                 in a registry package rather than copying it product to product."
            }
            Kind::Theme => {
                "Design tokens. This is usually the highest-value, lowest-risk thing to \
                 lift — it is declarative, it has no dependencies, and it carries the \
                 look that took longest to get right."
            }
            Kind::Art => {
                "Binary assets. Check licensing and provenance before reuse; these are \
                 the items most likely to carry someone else's rights."
            }
            Kind::Principle => {
                "Encoded judgment. Per principle 1b these are worth keeping even when no \
                 code moves — an undocumented decision gets re-litigated."
            }
            Kind::Ops => {
                "Scripts and workflows. Frequently reusable almost verbatim, and \
                 frequently full of hardcoded secrets and account IDs. Read before lifting."
            }
            Kind::Contract => {
                "Schemas and shared types. Candidates for a single declaration that both \
                 sides derive from, rather than two hand-written copies."
            }
            Kind::Test => {
                "The executable statement of what the logic does. Often the fastest route \
                 to understanding it, and the safety net if you port the logic."
            }
        }
    }
}

/// One inventoried file.
#[derive(Debug, Clone)]
pub struct Asset {
    pub rel_path: String,
    pub kind: Kind,
    pub bytes: u64,
    pub lines: usize,
    /// Why the classifier chose this kind — shown so a wrong guess is visible
    /// and correctable rather than silently authoritative.
    pub reason: &'static str,
    pub staged: bool,
}

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn run(source: &str, name: Option<String>, into: Option<String>, json: bool) -> Result<()> {
    let source_path = PathBuf::from(source);
    if !source_path.exists() {
        bail!(
            "`{source}` does not exist.\n\
             Pass a path to the repository or folder you want to harvest from."
        );
    }
    if !source_path.is_dir() {
        bail!("`{source}` is not a directory. Harvest works on a repository or folder.");
    }
    let source_path = source_path
        .canonicalize()
        .with_context(|| format!("resolving `{source}`"))?;

    let slug = match name {
        Some(n) => slugify(&n),
        None => slugify(
            &source_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "source".into()),
        ),
    };
    if slug.is_empty() {
        bail!("could not derive a name from `{source}` — pass one with --name");
    }

    // Refuse to harvest a directory into itself: the walk would consume its own
    // output, and the staged copies would be indistinguishable from the source.
    let dest_root = PathBuf::from(into.unwrap_or_else(|| "harvest".into())).join(&slug);
    let dest_abs = resolve(&dest_root)?;
    if source_path.starts_with(&dest_abs) || dest_abs.starts_with(&source_path) {
        bail!(
            "the destination `{}` is inside the source `{}`.\n\
             Harvest writes a staging copy, so the two must not overlap — the \n\
             walk would consume its own output.\n\
             Pass --into with a path outside the source tree.",
            dest_abs.display(),
            source_path.display()
        );
    }

    if !json {
        println!("✦ fid harvest — {}", source_path.display());
        println!();
    }

    let mut assets = walk(&source_path)?;
    assets.sort_by(|a, b| (a.kind, &a.rel_path).cmp(&(b.kind, &b.rel_path)));

    if assets.is_empty() {
        bail!(
            "nothing classifiable found under `{}`.\n\
             Every file was skipped as build output, a dependency, or an unknown type.",
            source_path.display()
        );
    }

    let stacks = detect_stacks(&source_path);

    // Stage reference copies for the agent to read.
    let assets = stage(&source_path, &dest_root, assets)?;

    write_inventory(&dest_root, &slug, &source_path, &stacks, &assets)?;
    write_survey(&dest_root, &slug, &source_path, &stacks, &assets)?;

    if json {
        print!("{}", json_report(&slug, &source_path, &stacks, &assets));
        return Ok(());
    }

    report(&dest_root, &slug, &stacks, &assets);
    Ok(())
}

// ── Walking and classification ───────────────────────────────────────────────

fn walk(root: &Path) -> Result<Vec<Asset>> {
    let mut assets = Vec::new();
    walk_dir(root, root, &mut assets)?;
    Ok(assets)
}

fn walk_dir(root: &Path, dir: &Path, out: &mut Vec<Asset>) -> Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        // An unreadable directory is a fact about the source, not a failure of
        // the survey — record nothing and continue.
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_dir() {
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk_dir(root, &path, out)?;
            continue;
        }

        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel_path = rel.to_string_lossy().replace('\\', "/");

        let Some((kind, reason)) = classify(&rel_path, &path) else {
            continue;
        };

        let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let lines = if is_text(&rel_path) {
            std::fs::read_to_string(&path)
                .map(|t| t.lines().count())
                .unwrap_or(0)
        } else {
            0
        };

        out.push(Asset {
            rel_path,
            kind,
            bytes,
            lines,
            reason,
            staged: false,
        });
    }
    Ok(())
}

/// Decide what a file is. Returns `None` for files that carry no reusable value
/// — lockfiles, editor state, OS detritus.
///
/// The order matters: the most specific signal wins. A `.css` file inside a
/// `components/` directory is theme material if it declares custom properties
/// and UI otherwise, so path and content are both consulted.
fn classify(rel_path: &str, abs: &Path) -> Option<(Kind, &'static str)> {
    let lower = rel_path.to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    let ext = file.rsplit_once('.').map(|(_, e)| e).unwrap_or("");

    // Noise: files that describe a machine's state, not a product's design.
    const NOISE_FILES: &[&str] = &[
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "cargo.lock",
        "poetry.lock",
        "gemfile.lock",
        "composer.lock",
        ".ds_store",
        "thumbs.db",
        ".env",
        ".env.local",
    ];
    if NOISE_FILES.contains(&file) || file.starts_with(".env.") {
        return None;
    }
    const NOISE_EXT: &[&str] = &["log", "tmp", "swp", "pyc", "class", "o", "a", "so", "dylib"];
    if NOISE_EXT.contains(&ext) {
        return None;
    }

    // ── Art ──────────────────────────────────────────────────────────────────
    const ART_EXT: &[&str] = &[
        "png", "jpg", "jpeg", "gif", "webp", "avif", "svg", "ico", "woff", "woff2", "ttf", "otf",
        "eot", "mp3", "wav", "ogg", "mp4", "webm", "glb", "gltf", "obj", "fbx", "blend", "psd",
        "ai", "sketch", "fig",
    ];
    if ART_EXT.contains(&ext) {
        return Some((Kind::Art, "binary or vector asset by extension"));
    }

    // ── Tests ────────────────────────────────────────────────────────────────
    if lower.contains(".test.")
        || lower.contains(".spec.")
        || lower.starts_with("test/")
        || lower.starts_with("tests/")
        || lower.contains("/tests/")
        || lower.contains("/__tests__/")
    {
        return Some((Kind::Test, "test path or filename convention"));
    }

    // ── Ops ──────────────────────────────────────────────────────────────────
    if lower.starts_with(".github/")
        || lower.starts_with("scripts/")
        || lower.contains("/scripts/")
        || lower.starts_with(".circleci/")
        || file == "dockerfile"
        || file == "docker-compose.yml"
        || file == "makefile"
        || ext == "tf"
        || file == "wrangler.jsonc"
        || file == "wrangler.toml"
        || file == "vercel.json"
        || file == "netlify.toml"
    {
        return Some((Kind::Ops, "build, deploy or infrastructure path"));
    }

    // ── Theme ────────────────────────────────────────────────────────────────
    if file.starts_with("tailwind.config")
        || lower.contains("tokens")
        || lower.contains("/theme")
        || file.starts_with("theme.")
        || file == "globals.css"
        || file == "app.css"
    {
        return Some((Kind::Theme, "design-token or theme path"));
    }
    if matches!(ext, "css" | "scss" | "sass" | "less" | "styl") {
        // A stylesheet declaring custom properties is a token source; one that
        // does not is styling for a specific surface.
        if let Ok(text) = std::fs::read_to_string(abs) {
            if text.contains("--") && (text.contains(":root") || text.contains("@theme")) {
                return Some((Kind::Theme, "stylesheet declaring CSS custom properties"));
            }
        }
        return Some((Kind::Ui, "stylesheet"));
    }

    // ── Contracts ────────────────────────────────────────────────────────────
    if lower.contains("migration")
        || lower.contains("schema")
        || ext == "sql"
        || ext == "proto"
        || ext == "graphql"
        || file.ends_with(".d.ts")
        || file == "openapi.yaml"
        || file == "openapi.json"
    {
        return Some((
            Kind::Contract,
            "schema, migration or shared type declaration",
        ));
    }

    // ── Principles ───────────────────────────────────────────────────────────
    if matches!(ext, "md" | "mdx" | "rst" | "adr" | "txt") {
        return Some((
            Kind::Principle,
            "prose — conventions, decisions, documentation",
        ));
    }

    // ── Markup ───────────────────────────────────────────────────────────────
    // HTML is UI, and for a landing page it is the whole deliverable — the
    // structure, the copy and the layout all live here. Omitting it made
    // `fid harvest` useless for the most common donor there is: a static site.
    if matches!(
        ext,
        "html" | "htm" | "hbs" | "ejs" | "pug" | "liquid" | "njk" | "twig" | "erb"
    ) {
        return Some((Kind::Ui, "markup — page structure, copy and layout"));
    }

    // ── UI vs logic ──────────────────────────────────────────────────────────
    const CODE_EXT: &[&str] = &[
        "ts", "tsx", "js", "jsx", "mjs", "cjs", "svelte", "vue", "rs", "py", "go", "rb", "java",
        "kt", "swift", "c", "cpp", "h", "hpp", "cs", "php", "ex", "exs",
    ];
    if !CODE_EXT.contains(&ext) {
        // Config formats carry real decisions, but only at the root of a tree.
        if matches!(ext, "json" | "yaml" | "yml" | "toml") {
            return Some((Kind::Ops, "configuration"));
        }
        return None;
    }

    if matches!(ext, "tsx" | "jsx" | "svelte" | "vue") {
        return Some((Kind::Ui, "component file extension"));
    }
    if lower.contains("/components/")
        || lower.starts_with("components/")
        || lower.contains("/ui/")
        || lower.contains("/pages/")
        || lower.contains("/routes/")
        || lower.contains("/views/")
    {
        return Some((Kind::Ui, "component, page or route path"));
    }

    // Remaining code: logic if it imports no framework and performs no I/O.
    // This is a heuristic and is reported as one — `reason` is printed so a
    // wrong call is visible rather than authoritative.
    if let Ok(text) = std::fs::read_to_string(abs) {
        let framework = [
            "from 'react'",
            "from \"react\"",
            "from 'svelte'",
            "from \"svelte\"",
            "from 'vue'",
            "next/",
            "@angular/",
        ]
        .iter()
        .any(|needle| text.contains(needle));

        let io = [
            "fetch(",
            "axios",
            "fs.read",
            "fs.write",
            "require('fs')",
            "createClient",
            "process.env",
        ]
        .iter()
        .any(|needle| text.contains(needle));

        return match (framework, io) {
            (false, false) => Some((
                Kind::Logic,
                "code with no framework import and no I/O call — likely pure",
            )),
            (true, _) => Some((Kind::Ui, "code importing a UI framework")),
            (false, true) => Some((
                Kind::Logic,
                "code with no framework import but performing I/O — logic mixed with effects",
            )),
        };
    }

    Some((Kind::Logic, "source file"))
}

fn is_text(rel_path: &str) -> bool {
    let ext = rel_path.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    !matches!(
        ext,
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "avif"
            | "ico"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "mp3"
            | "wav"
            | "mp4"
            | "webm"
            | "glb"
            | "psd"
    )
}

// ── Stack detection ──────────────────────────────────────────────────────────

/// Which frameworks and tools the donor used, read from its manifests.
///
/// This tells the agent what a file's idioms *mean* — a `+page.svelte` is a
/// route only if the project is SvelteKit — and it tells a human how much
/// translation the port will need.
fn detect_stacks(root: &Path) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();

    if let Ok(pkg) = std::fs::read_to_string(root.join("package.json")) {
        for (needle, label) in [
            ("\"next\"", "Next.js"),
            ("\"react\"", "React"),
            ("\"svelte\"", "Svelte"),
            ("\"@sveltejs/kit\"", "SvelteKit"),
            ("\"vue\"", "Vue"),
            ("\"astro\"", "Astro"),
            ("\"tailwindcss\"", "Tailwind CSS"),
            ("\"@supabase/supabase-js\"", "Supabase"),
            ("\"drizzle-orm\"", "Drizzle"),
            ("\"prisma\"", "Prisma"),
            ("\"vitest\"", "Vitest"),
            ("\"playwright\"", "Playwright"),
            ("\"turbo\"", "Turborepo"),
            ("\"three\"", "Three.js"),
        ] {
            if pkg.contains(needle) {
                found.push(label.to_string());
            }
        }
    }
    if root.join("Cargo.toml").is_file() {
        found.push("Rust / Cargo".into());
    }
    if root.join("pnpm-workspace.yaml").is_file() {
        found.push("pnpm workspace".into());
    }
    if root.join("go.mod").is_file() {
        found.push("Go".into());
    }
    if root.join("pyproject.toml").is_file() || root.join("requirements.txt").is_file() {
        found.push("Python".into());
    }
    if root.join("supabase").is_dir() {
        found.push("Supabase (local config)".into());
    }
    if root.join("wrangler.jsonc").is_file()
        || root.join("wrangler.toml").is_file()
        || root.join("wrangler.json").is_file()
    {
        found.push("Cloudflare Workers".into());
    }
    if root.join("index.html").is_file() || root.join("site").is_dir() {
        found.push("Static site".into());
    }

    found.sort();
    found.dedup();
    found
}

// ── Staging ──────────────────────────────────────────────────────────────────

/// Copy reference material into the staging tree.
///
/// Only text under [`MAX_STAGE_BYTES`] is copied. Large binaries are inventoried
/// but left where they are: the point of staging is to give an agent something
/// it can *read*, and no agent reads a 40 MB PSD.
fn stage(source: &Path, dest_root: &Path, mut assets: Vec<Asset>) -> Result<Vec<Asset>> {
    let assets_dir = dest_root.join("assets");
    std::fs::create_dir_all(&assets_dir)
        .with_context(|| format!("creating `{}`", assets_dir.display()))?;

    for asset in &mut assets {
        if asset.bytes > MAX_STAGE_BYTES || !is_text(&asset.rel_path) {
            continue;
        }
        let from = source.join(&asset.rel_path);
        let to = assets_dir.join(&asset.rel_path);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        if std::fs::copy(&from, &to).is_ok() {
            asset.staged = true;
        }
    }

    Ok(assets)
}

// ── Output ───────────────────────────────────────────────────────────────────

fn by_kind(assets: &[Asset]) -> BTreeMap<Kind, Vec<&Asset>> {
    let mut map: BTreeMap<Kind, Vec<&Asset>> = BTreeMap::new();
    for asset in assets {
        map.entry(asset.kind).or_default().push(asset);
    }
    map
}

/// The machine-readable inventory. This is what an agent reads.
fn write_inventory(
    dest_root: &Path,
    slug: &str,
    source: &Path,
    stacks: &[String],
    assets: &[Asset],
) -> Result<()> {
    let mut out = String::new();
    out.push_str(
        "# harvest.toml — machine-readable inventory of a donor codebase.\n\
         #\n\
         # Written by `fid harvest`. Regenerate rather than editing: this is a\n\
         # survey of another repository, not a declaration of your own.\n\
         #\n\
         # `kind` is a heuristic and `reason` states the evidence for it. Treat a\n\
         # classification as a starting point, not a verdict.\n\n",
    );

    let _ = writeln!(out, "[source]");
    let _ = writeln!(out, "name = {:?}", slug);
    let _ = writeln!(out, "path = {:?}", source.display().to_string());
    let _ = writeln!(out, "stacks = {:?}", stacks);
    let _ = writeln!(out, "file_count = {}", assets.len());
    let _ = writeln!(
        out,
        "total_lines = {}",
        assets.iter().map(|a| a.lines).sum::<usize>()
    );
    out.push('\n');

    for (kind, group) in by_kind(assets) {
        let _ = writeln!(out, "[summary.{}]", kind.as_str());
        let _ = writeln!(out, "count = {}", group.len());
        let _ = writeln!(
            out,
            "lines = {}",
            group.iter().map(|a| a.lines).sum::<usize>()
        );
        let _ = writeln!(out, "note = {:?}", kind.why());
        out.push('\n');
    }

    for asset in assets {
        let _ = writeln!(out, "[[asset]]");
        let _ = writeln!(out, "path = {:?}", asset.rel_path);
        let _ = writeln!(out, "kind = {:?}", asset.kind.as_str());
        let _ = writeln!(out, "lines = {}", asset.lines);
        let _ = writeln!(out, "bytes = {}", asset.bytes);
        let _ = writeln!(out, "reason = {:?}", asset.reason);
        let _ = writeln!(out, "staged = {}", asset.staged);
        out.push('\n');
    }

    let path = dest_root.join("harvest.toml");
    std::fs::write(&path, out).with_context(|| format!("writing `{}`", path.display()))?;
    Ok(())
}

/// The human-readable survey, and the agent's brief.
fn write_survey(
    dest_root: &Path,
    slug: &str,
    source: &Path,
    stacks: &[String],
    assets: &[Asset],
) -> Result<()> {
    let mut out = String::new();
    let _ = writeln!(out, "# Harvest survey — `{slug}`\n");
    let _ = writeln!(
        out,
        "> Source: `{}`  \n\
         > Surveyed by `fid harvest`. **Nothing here is wired into this product.**\n",
        source.display()
    );

    out.push_str(
        "This directory is a staging area, not source. Files under `assets/` are \
         reference copies of another codebase. Read them, decide what is worth \
         keeping, and *generalize* it into this product deliberately — do not \
         paste them in. A pasted copy is a second declaration of somebody else's \
         assumptions, which is the thing this platform exists to delete.\n\n",
    );

    if !stacks.is_empty() {
        let _ = writeln!(out, "## Donor stack\n");
        for stack in stacks {
            let _ = writeln!(out, "- {stack}");
        }
        out.push('\n');
        out.push_str(
            "This matters for translation: an idiom only means what its framework \
             makes it mean, and the further the donor's stack is from the target's, \
             the more of what looks like logic is actually framing.\n\n",
        );
    }

    let grouped = by_kind(assets);

    let _ = writeln!(out, "## What is here\n");
    let _ = writeln!(out, "| Kind | Files | Lines | Why it matters |");
    let _ = writeln!(out, "|---|---:|---:|---|");
    for (kind, group) in &grouped {
        let _ = writeln!(
            out,
            "| **{}** | {} | {} | {} |",
            kind.as_str(),
            group.len(),
            group.iter().map(|a| a.lines).sum::<usize>(),
            kind.why()
        );
    }
    out.push('\n');

    let _ = writeln!(out, "## Suggested order of work\n");
    out.push_str(
        "Ordered by value per unit of risk, not by size:\n\n\
         1. **theme** — declarative, dependency-free, and carries the look that took \
            longest to get right. Lift it first; it is almost always worth it.\n\
         2. **principle** — costs nothing to keep and prevents re-litigating decisions \
            someone already made carefully (principle 1b).\n\
         3. **contract** — schemas and shared types are candidates for a *single* \
            declaration both sides derive from, which is usually an improvement on \
            what the donor had.\n\
         4. **logic** — the highest value and the highest care. Port the tests with it, \
            or port nothing.\n\
         5. **ui** — generalize the shape, drop the product's nouns.\n\
         6. **ops** — often reusable nearly verbatim, and often full of hardcoded \
            account IDs and secrets. Read every line.\n\
         7. **art** — check licensing and provenance before anything else.\n\n",
    );

    for (kind, group) in &grouped {
        let _ = writeln!(out, "## {} ({} files)\n", kind.as_str(), group.len());
        let mut shown: Vec<&&Asset> = group.iter().collect();
        shown.sort_by_key(|a| std::cmp::Reverse(a.lines));
        let _ = writeln!(out, "| File | Lines | Staged | Classified because |");
        let _ = writeln!(out, "|---|---:|:-:|---|");
        for asset in shown.iter().take(40) {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} |",
                asset.rel_path,
                asset.lines,
                if asset.staged { "✓" } else { "—" },
                asset.reason
            );
        }
        if group.len() > 40 {
            let _ = writeln!(
                out,
                "\n_{} more — see `harvest.toml` for the full inventory._",
                group.len() - 40
            );
        }
        out.push('\n');
    }

    out.push_str(
        "## Next\n\n\
         Run the `/fiducial:harvest` skill in this repository. It reads \
         `harvest.toml`, reads the staged sources, and walks the extraction \
         decisions kind by kind — including the ones where the right answer is \
         \"do not lift this\".\n",
    );

    let path = dest_root.join("SURVEY.md");
    std::fs::write(&path, out).with_context(|| format!("writing `{}`", path.display()))?;
    Ok(())
}

fn json_report(slug: &str, source: &Path, stacks: &[String], assets: &[Asset]) -> String {
    let mut out = String::from("{\n");
    let _ = writeln!(out, "  \"name\": {:?},", slug);
    let _ = writeln!(out, "  \"source\": {:?},", source.display().to_string());
    let _ = writeln!(
        out,
        "  \"stacks\": [{}],",
        stacks
            .iter()
            .map(|s| format!("{s:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(out, "  \"file_count\": {},", assets.len());

    out.push_str("  \"summary\": {\n");
    let grouped = by_kind(assets);
    let rows: Vec<String> = grouped
        .iter()
        .map(|(kind, group)| {
            format!(
                "    {:?}: {{ \"count\": {}, \"lines\": {} }}",
                kind.as_str(),
                group.len(),
                group.iter().map(|a| a.lines).sum::<usize>()
            )
        })
        .collect();
    out.push_str(&rows.join(",\n"));
    out.push_str("\n  },\n");

    out.push_str("  \"assets\": [\n");
    let rows: Vec<String> = assets
        .iter()
        .map(|a| {
            format!(
                "    {{ \"path\": {:?}, \"kind\": {:?}, \"lines\": {}, \"bytes\": {}, \"staged\": {}, \"reason\": {:?} }}",
                a.rel_path,
                a.kind.as_str(),
                a.lines,
                a.bytes,
                a.staged,
                a.reason
            )
        })
        .collect();
    out.push_str(&rows.join(",\n"));
    out.push_str("\n  ]\n}\n");
    out
}

fn report(dest_root: &Path, slug: &str, stacks: &[String], assets: &[Asset]) {
    if !stacks.is_empty() {
        println!("  Donor stack");
        for stack in stacks {
            println!("    · {stack}");
        }
        println!();
    }

    println!("  Inventory");
    for (kind, group) in by_kind(assets) {
        let lines: usize = group.iter().map(|a| a.lines).sum();
        println!(
            "    {:<10} {:>4} files  {:>7} lines",
            kind.as_str(),
            group.len(),
            lines
        );
    }
    println!();

    let staged = assets.iter().filter(|a| a.staged).count();
    println!("  Staged   {staged} of {} files into", assets.len());
    println!("           {}/assets/", dest_root.display());
    println!();
    println!(
        "  Wrote    {}/harvest.toml   (machine-readable inventory)",
        dest_root.display()
    );
    println!(
        "           {}/SURVEY.md      (read this first)",
        dest_root.display()
    );
    println!();
    println!("  ⚠ Nothing was wired into this product. `harvest/` is a staging");
    println!("    area — the donor is reference material until you decide");
    println!("    what to lift and how to generalize it.");
    println!();
    println!("✦ next: run `/fiducial:harvest {slug}` to work through the extraction.");
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Absolute, normalized form of a path that **need not exist yet**.
///
/// `Path::canonicalize` fails on a missing path, which silently disabled the
/// overlap check above: the destination never exists on a first run, so the
/// guard was skipped exactly when it was needed. This resolves against the
/// nearest existing ancestor instead, and normalizes `.` and `..` by hand.
fn resolve(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("getting current directory")?
            .join(path)
    };

    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }

    // Resolve symlinks on whatever prefix already exists, so a symlinked source
    // and a literal destination still compare correctly.
    let mut existing = out.clone();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    while !existing.exists() {
        match existing.file_name() {
            Some(name) => {
                tail.push(name.to_os_string());
                existing.pop();
            }
            None => break,
        }
    }
    if let Ok(real) = existing.canonicalize() {
        out = real;
        for name in tail.into_iter().rev() {
            out.push(name);
        }
    }

    Ok(out)
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_produces_a_clean_directory_name() {
        assert_eq!(slugify("Ring of Pursuit"), "ring-of-pursuit");
        assert_eq!(slugify("my_app.v2"), "my-app-v2");
        assert_eq!(slugify("---"), "");
        assert_eq!(slugify("already-clean"), "already-clean");
    }

    #[test]
    fn art_is_classified_by_extension() {
        let (kind, _) = classify("public/logo.svg", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Art);
        let (kind, _) = classify("assets/font.woff2", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Art);
    }

    #[test]
    fn tests_win_over_the_extension_that_would_call_them_ui() {
        // A .tsx file is a component by extension, but a test path is stronger
        // evidence — otherwise every component test lands in the UI bucket.
        let (kind, _) = classify("src/Button.test.tsx", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Test);
    }

    #[test]
    fn lockfiles_and_env_files_are_not_assets() {
        assert!(classify("pnpm-lock.yaml", Path::new("/nonexistent")).is_none());
        assert!(classify(".env.production", Path::new("/nonexistent")).is_none());
        assert!(classify("debug.log", Path::new("/nonexistent")).is_none());
    }

    #[test]
    fn component_paths_are_ui_even_without_a_component_extension() {
        let (kind, _) = classify("src/components/helpers.ts", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Ui);
    }

    #[test]
    fn workflows_and_scripts_are_ops() {
        let (kind, _) = classify(".github/workflows/ci.yml", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Ops);
        let (kind, _) = classify("scripts/deploy.mjs", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Ops);
    }

    #[test]
    fn tool_scratch_directories_are_skipped() {
        // Running this command on a real monorepo put a 13,206-line VitePress
        // dependency chunk at the top of the "logic" findings, ahead of every
        // real rule in the codebase. Vendored caches are not the donor's work.
        for dir in [
            ".vitepress",
            ".wrangler",
            ".temp",
            "cache",
            "storybook-static",
            "test-results",
        ] {
            assert!(
                SKIP_DIRS.contains(&dir),
                "`{dir}` should never be walked"
            );
        }
    }

    #[test]
    fn markup_is_ui() {
        // A landing page IS its HTML. Before this rule `fid harvest` classified
        // nothing at all in a static site, which is the commonest donor there is.
        let (kind, _) = classify("site/index.html", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Ui);
        let (kind, _) = classify("LandingHero.dc.html", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Ui);
    }

    #[test]
    fn prose_is_a_principle() {
        let (kind, _) = classify("docs/decisions/0001-why.md", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Principle);
    }

    #[test]
    fn theme_paths_beat_the_generic_stylesheet_rule() {
        let (kind, _) = classify("src/styles/tokens.ts", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Theme);
        let (kind, _) = classify("tailwind.config.ts", Path::new("/nonexistent")).unwrap();
        assert_eq!(kind, Kind::Theme);
    }

    #[test]
    fn every_kind_states_why_it_matters() {
        // The survey prints `why()` for each kind present. An empty one would
        // render a blank table cell and tell the reader nothing.
        for kind in [
            Kind::Logic,
            Kind::Ui,
            Kind::Theme,
            Kind::Art,
            Kind::Principle,
            Kind::Ops,
            Kind::Contract,
            Kind::Test,
        ] {
            assert!(!kind.why().is_empty(), "{} has no rationale", kind.as_str());
            assert!(!kind.as_str().is_empty());
        }
    }
}
