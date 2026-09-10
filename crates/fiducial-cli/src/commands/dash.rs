//! `fid dash [--json] [--section <name>]` — the workbench, read-only.
//!
//! **The repository is the database; this is a view.** It owns no store, caches
//! nothing, and writes nothing. Every number it prints is recomputed from files
//! already in the repo, so it cannot be stale in the way a synced dashboard can
//! — and there is nothing to invalidate when the repo changes.
//!
//! That constraint is what keeps it honest, and it shapes two decisions:
//!
//! - **No network by default.** CI is reported from the workflow files the repo
//!   declares, not from a live API. A view that needs a token and a connection
//!   to render is a view that stops working on a plane, and a cached answer
//!   would be a private store by another name.
//! - **Absence is a finding, not an error.** A product with no roadmap, no
//!   decisions, or no pipelines gets a section saying exactly that. Reporting
//!   "not declared" is more useful than refusing to render.
//!
//! `--json` emits the same facts for agents and for workbench v1, so neither
//! needs a second implementation of the same reads.

use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    config::{Config, CONFIG_FILE},
    lock::{sha256_hex, Lock, LOCK_FILE},
    pipeline,
};

// ── The view ──────────────────────────────────────────────────────────────────

/// Everything `fid dash` reports, in one serialisable value.
#[derive(Debug, Serialize)]
pub struct Dash {
    product: ProductView,
    git: GitView,
    roadmap: RoadmapView,
    decisions: DecisionsView,
    ci: CiView,
    graph: GraphView,
    freshness: FreshnessView,
}

#[derive(Debug, Serialize)]
struct ProductView {
    name: String,
    version: String,
    root: String,
    spine_enabled: bool,
    capabilities: Vec<String>,
    guard_rules: usize,
}

#[derive(Debug, Serialize)]
struct GitView {
    /// Whether the product root is inside a git work tree.
    ///
    /// Tracked separately from `branch` because a freshly initialised repo has
    /// an unborn HEAD: it is unmistakably a repository, but has no commit and so
    /// no branch to resolve. Inferring "not a repository" from a missing branch
    /// reported every newly scaffolded product as un-versioned.
    is_repo: bool,
    /// `None` before the first commit.
    branch: Option<String>,
    commit: Option<String>,
    subject: Option<String>,
    /// Files with uncommitted changes.
    dirty_files: usize,
    /// Commits ahead of / behind the tracking branch, when one is configured.
    ahead: Option<u32>,
    behind: Option<u32>,
}

/// Roadmap progress, counted from status markers in a roadmap file.
#[derive(Debug, Serialize)]
struct RoadmapView {
    /// Which file it was read from, relative to the root.
    source: Option<String>,
    done: usize,
    in_progress: usize,
    todo: usize,
    /// Lines marked in-progress, which is what someone opening a dashboard
    /// wants to see first.
    active: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DecisionsView {
    /// Directory it read, relative to the root.
    source: Option<String>,
    count: usize,
    /// Most recent first, by the date each filename is prefixed with.
    recent: Vec<Decision>,
}

#[derive(Debug, Serialize)]
struct Decision {
    file: String,
    date: Option<String>,
    title: String,
}

#[derive(Debug, Serialize)]
struct CiView {
    workflows: Vec<Workflow>,
}

#[derive(Debug, Serialize)]
struct Workflow {
    file: String,
    name: String,
    /// Event names the workflow declares a trigger for.
    triggers: Vec<String>,
    /// Whether it runs `fid derive --check`, the guard against stale artifacts.
    checks_freshness: bool,
}

#[derive(Debug, Serialize)]
struct GraphView {
    pipelines: Vec<PipelineSummary>,
}

#[derive(Debug, Serialize)]
struct PipelineSummary {
    name: String,
    executor: String,
    outputs: Vec<String>,
}

/// Artifact and template freshness, recomputed by hashing.
#[derive(Debug, Serialize)]
struct FreshnessView {
    artifacts_fresh: usize,
    artifacts_stale: usize,
    artifacts_missing: usize,
    artifacts_untracked: usize,
    templates_modified: usize,
    /// One line per problem, ready to print.
    problems: Vec<String>,
}

// ── Collection ────────────────────────────────────────────────────────────────

/// Roadmap files looked for, in order of preference.
const ROADMAP_FILES: &[&str] = &["ROADMAP.md", "PHASES.md", "docs/ROADMAP.md"];

/// Directories a product may keep dated decision records in.
const DECISION_DIRS: &[&str] = &["docs/specs", "docs/decisions", "docs/adr"];

/// How many decisions the text view lists before it stops.
const RECENT_DECISIONS: usize = 5;

impl Dash {
    /// Read every section from the repository.
    pub fn collect(root: &Path, config: &Config) -> Result<Self> {
        Ok(Self {
            product: ProductView {
                name: config.product.name.clone(),
                version: config.product.version.clone(),
                root: root.display().to_string(),
                spine_enabled: config.spine.enabled,
                capabilities: config.capabilities.enabled.clone(),
                guard_rules: config.guard.rules.len(),
            },
            git: git_view(root),
            roadmap: roadmap_view(root),
            decisions: decisions_view(root),
            ci: ci_view(root),
            graph: GraphView {
                pipelines: summarise(pipeline::discover(root, config)?),
            },
            // Freshness needs the declared outputs as well as the lock, so it
            // can report an output nothing has derived yet — invisible if you
            // only look at what the lock already tracks.
            freshness: freshness_view(root, &pipeline::discover(root, config)?)?,
        })
    }
}

/// Run a git command in `root`, returning trimmed stdout on success.
///
/// Every failure — git absent, not a repository, no commits yet — collapses to
/// `None`. A dashboard that refuses to render because a product has no commits
/// is worse than one that says so.
fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn git_view(root: &Path) -> GitView {
    let is_repo = git(root, &["rev-parse", "--is-inside-work-tree"]).as_deref() == Some("true");
    if !is_repo {
        return GitView {
            is_repo: false,
            branch: None,
            commit: None,
            subject: None,
            dirty_files: 0,
            ahead: None,
            behind: None,
        };
    }

    let dirty_files = git(root, &["status", "--porcelain"])
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);

    // `rev-list --count --left-right` prints "<ahead>\t<behind>" against the
    // upstream, and fails when no upstream is configured — which is normal for
    // a freshly scaffolded product.
    let (ahead, behind) = match git(
        root,
        &["rev-list", "--count", "--left-right", "@{u}...HEAD"],
    ) {
        Some(s) => {
            let mut parts = s.split_whitespace();
            let behind = parts.next().and_then(|v| v.parse().ok());
            let ahead = parts.next().and_then(|v| v.parse().ok());
            (ahead, behind)
        }
        None => (None, None),
    };

    GitView {
        is_repo: true,
        // `rev-parse --abbrev-ref` fails on an unborn HEAD; `symbolic-ref`
        // still reports the branch a first commit would land on.
        branch: git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
            .or_else(|| git(root, &["symbolic-ref", "--short", "HEAD"])),
        commit: git(root, &["rev-parse", "--short", "HEAD"]),
        subject: git(root, &["log", "-1", "--pretty=%s"]),
        dirty_files,
        ahead,
        behind,
    }
}

/// Classify a roadmap line by the status marker it carries.
///
/// Recognises both the emoji table markers this project uses and GitHub task
/// list syntax, because a product's roadmap is its own to format. A line with no
/// marker is not a roadmap item and is ignored — which is what keeps prose from
/// being counted.
fn roadmap_status(line: &str) -> Option<&'static str> {
    if line.contains("- [x]") || line.contains("- [X]") {
        return Some("done");
    }
    if line.contains("- [ ]") {
        return Some("todo");
    }
    // Emoji markers, checked before the plain-text ones so a row containing
    // both is counted by its marker.
    if line.contains('✅') {
        return Some("done");
    }
    if line.contains('🟡') || line.contains("🚧") {
        return Some("in_progress");
    }
    if line.contains('⬜') || line.contains("⏸") {
        return Some("todo");
    }
    None
}

/// Strip table pipes and markers so an active line reads as a sentence.
fn tidy_roadmap_line(line: &str) -> String {
    let cleaned: String = line
        .replace('|', " ")
        .replace("- [ ]", " ")
        .replace("- [x]", " ")
        .replace('🟡', " ")
        .replace("🚧", " ")
        .chars()
        .collect();
    let mut out = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.len() > 96 {
        out.truncate(93);
        out.push_str("...");
    }
    out
}

fn roadmap_view(root: &Path) -> RoadmapView {
    let found = ROADMAP_FILES
        .iter()
        .map(|f| (f, root.join(f)))
        .find(|(_, p)| p.is_file());

    let Some((rel, path)) = found else {
        return RoadmapView {
            source: None,
            done: 0,
            in_progress: 0,
            todo: 0,
            active: Vec::new(),
        };
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return RoadmapView {
            source: None,
            done: 0,
            in_progress: 0,
            todo: 0,
            active: Vec::new(),
        };
    };

    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut active = Vec::new();
    for line in text.lines() {
        if let Some(status) = roadmap_status(line) {
            *counts.entry(status).or_insert(0) += 1;
            if status == "in_progress" {
                active.push(tidy_roadmap_line(line));
            }
        }
    }

    RoadmapView {
        source: Some((*rel).to_string()),
        done: counts.get("done").copied().unwrap_or(0),
        in_progress: counts.get("in_progress").copied().unwrap_or(0),
        todo: counts.get("todo").copied().unwrap_or(0),
        active,
    }
}

/// Leading `YYYY-MM-DD` of a filename, when it has one.
fn date_prefix(name: &str) -> Option<String> {
    let bytes = name.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let head = &name[..10];
    let shaped = head.chars().enumerate().all(|(i, c)| {
        if i == 4 || i == 7 {
            c == '-'
        } else {
            c.is_ascii_digit()
        }
    });
    shaped.then(|| head.to_string())
}

/// First markdown heading in a file, falling back to its stem.
fn markdown_title(path: &Path, fallback: &str) -> String {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| {
            t.lines()
                .find(|l| l.starts_with("# "))
                .map(|l| l.trim_start_matches("# ").trim().to_string())
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn decisions_view(root: &Path) -> DecisionsView {
    let found = DECISION_DIRS
        .iter()
        .map(|d| (d, root.join(d)))
        .find(|(_, p)| p.is_dir());

    let Some((rel, dir)) = found else {
        return DecisionsView {
            source: None,
            count: 0,
            recent: Vec::new(),
        };
    };

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    // Filenames are date-prefixed by the append-only rule, so name order is
    // chronological order. Newest first.
    files.sort();
    files.reverse();

    let count = files.len();
    let recent = files
        .iter()
        .take(RECENT_DECISIONS)
        .map(|p| {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            Decision {
                date: date_prefix(&name),
                title: markdown_title(p, &name),
                file: name,
            }
        })
        .collect();

    DecisionsView {
        source: Some((*rel).to_string()),
        count,
        recent,
    }
}

fn ci_view(root: &Path) -> CiView {
    let dir = root.join(".github/workflows");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .collect();
    files.sort();

    let workflows = files
        .iter()
        .filter_map(|p| {
            let text = std::fs::read_to_string(p).ok()?;
            let file = p.file_name()?.to_str()?.to_string();
            // Parsed by line rather than with a YAML crate: the two fields
            // needed are top-level, and a workflow whose triggers this misreads
            // is still reported by name. Pulling in a YAML parser to read two
            // keys would be the larger mistake.
            let name = text
                .lines()
                .find(|l| l.starts_with("name:"))
                .map(|l| {
                    l.trim_start_matches("name:")
                        .trim()
                        .trim_matches('"')
                        .to_string()
                })
                .unwrap_or_else(|| file.clone());
            let triggers = ["push", "pull_request", "workflow_dispatch", "schedule"]
                .iter()
                .filter(|t| text.contains(&format!("  {t}:")) || text.contains(&format!("{t}:\n")))
                .map(|t| (*t).to_string())
                .collect();
            Some(Workflow {
                checks_freshness: text.contains("derive --check"),
                file,
                name,
                triggers,
            })
        })
        .collect();

    CiView { workflows }
}

fn summarise(pipelines: Vec<pipeline::Pipeline>) -> Vec<PipelineSummary> {
    pipelines
        .into_iter()
        .map(|p| PipelineSummary {
            name: p.name,
            executor: p.executor,
            outputs: p.outputs,
        })
        .collect()
}

fn freshness_view(root: &Path, pipelines: &[pipeline::Pipeline]) -> Result<FreshnessView> {
    let lock_path = root.join(LOCK_FILE);
    if !lock_path.exists() {
        return Ok(FreshnessView {
            artifacts_fresh: 0,
            artifacts_stale: 0,
            artifacts_missing: 0,
            artifacts_untracked: 0,
            templates_modified: 0,
            problems: vec![format!("{LOCK_FILE} is missing — run `fid derive`")],
        });
    }
    let lock = Lock::load(&lock_path).with_context(|| format!("reading {LOCK_FILE}"))?;

    let mut view = FreshnessView {
        artifacts_fresh: 0,
        artifacts_stale: 0,
        artifacts_missing: 0,
        artifacts_untracked: 0,
        templates_modified: 0,
        problems: Vec::new(),
    };

    for (path, record) in &lock.artifacts {
        match std::fs::read(root.join(path)) {
            Err(_) => {
                view.artifacts_missing += 1;
                view.problems
                    .push(format!("{path}: missing — run `fid derive`"));
            }
            Ok(content) => {
                if sha256_hex(&content) == record.hash {
                    view.artifacts_fresh += 1;
                } else {
                    view.artifacts_stale += 1;
                    view.problems
                        .push(format!("{path}: stale — run `fid derive`"));
                }
            }
        }
    }

    // An output a pipeline declares but the lock has never recorded has not
    // been derived yet. `fid derive --check` reports this too, but only for
    // outputs it is asked about; here it is a standing count.
    for p in pipelines {
        for out in &p.outputs {
            if !lock.artifacts.contains_key(out.as_str()) {
                view.artifacts_untracked += 1;
                view.problems
                    .push(format!("{out}: never derived — run `fid derive`"));
            }
        }
    }

    // A template whose content no longer matches its recorded hash has been
    // hand-edited. That is not necessarily wrong — but `fid upgrade` will have
    // to merge around it, so it belongs on a dashboard.
    for (path, record) in &lock.templates {
        if let Ok(content) = std::fs::read(root.join(path)) {
            if sha256_hex(&content) != record.hash {
                view.templates_modified += 1;
            }
        }
    }

    Ok(view)
}

// ── Text rendering ────────────────────────────────────────────────────────────

/// Section names `--section` accepts.
pub const SECTIONS: &[&str] = &[
    "product",
    "git",
    "roadmap",
    "decisions",
    "ci",
    "graph",
    "freshness",
];

fn heading(title: &str) {
    println!("\n\x1b[1m{title}\x1b[0m");
}

fn field(label: &str, value: impl std::fmt::Display) {
    println!("  {label:<14} {value}");
}

impl Dash {
    /// Render as text, or one section of it.
    ///
    /// Sections print in a fixed order so the output is diffable between runs —
    /// a dashboard whose lines move around is hard to read twice.
    pub fn render(&self, only: Option<&str>) {
        let want = |s: &str| only.is_none_or(|o| o == s);

        if want("product") {
            heading("Product");
            field("name", &self.product.name);
            field("version", &self.product.version);
            field("root", &self.product.root);
            field(
                "spine",
                if self.product.spine_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
            );
            field(
                "capabilities",
                if self.product.capabilities.is_empty() {
                    "(none)".to_string()
                } else {
                    self.product.capabilities.join(", ")
                },
            );
            field("guard rules", self.product.guard_rules);
        }

        if want("git") {
            heading("Git");
            if !self.git.is_repo {
                field("state", "not a git repository");
            } else {
                match &self.git.branch {
                    Some(branch) => field("branch", branch),
                    None => field("branch", "(unresolved)"),
                }
                match (&self.git.commit, &self.git.subject) {
                    (Some(c), Some(s)) => field("head", format!("{c}  {s}")),
                    _ => field("head", "no commits yet"),
                }
                field(
                    "working tree",
                    if self.git.dirty_files == 0 {
                        "clean".to_string()
                    } else {
                        format!("{} file(s) with uncommitted changes", self.git.dirty_files)
                    },
                );
                match (self.git.ahead, self.git.behind) {
                    (Some(a), Some(b)) => field("upstream", format!("{a} ahead, {b} behind")),
                    _ => field("upstream", "not tracking a remote branch"),
                }
            }
        }

        if want("roadmap") {
            heading("Roadmap");
            match &self.roadmap.source {
                None => field(
                    "state",
                    format!("no roadmap found (looked for {})", ROADMAP_FILES.join(", ")),
                ),
                Some(src) => {
                    field("source", src);
                    let total = self.roadmap.done + self.roadmap.in_progress + self.roadmap.todo;
                    field(
                        "progress",
                        format!(
                            "{} done, {} in progress, {} to do  ({total} tracked)",
                            self.roadmap.done, self.roadmap.in_progress, self.roadmap.todo
                        ),
                    );
                    for line in &self.roadmap.active {
                        println!("  → {line}");
                    }
                }
            }
        }

        if want("decisions") {
            heading("Decisions");
            match &self.decisions.source {
                None => field(
                    "state",
                    format!("none recorded (looked in {})", DECISION_DIRS.join(", ")),
                ),
                Some(src) => {
                    field(
                        "source",
                        format!("{src}  ({} recorded)", self.decisions.count),
                    );
                    for d in &self.decisions.recent {
                        let date = d.date.clone().unwrap_or_else(|| "----------".into());
                        println!("  {date}  {}", d.title);
                    }
                    if self.decisions.count > self.decisions.recent.len() {
                        println!(
                            "  … {} older",
                            self.decisions.count - self.decisions.recent.len()
                        );
                    }
                }
            }
        }

        if want("ci") {
            heading("CI");
            if self.ci.workflows.is_empty() {
                field("state", "no workflows in .github/workflows");
            } else {
                for w in &self.ci.workflows {
                    let triggers = if w.triggers.is_empty() {
                        "no recognised trigger".to_string()
                    } else {
                        w.triggers.join(", ")
                    };
                    let guard = if w.checks_freshness {
                        "  [checks artifact freshness]"
                    } else {
                        ""
                    };
                    println!("  {:<28} on {triggers}{guard}", w.name);
                }
                if !self.ci.workflows.iter().any(|w| w.checks_freshness) {
                    println!("  note: no workflow runs `fid derive --check`, so a stale");
                    println!("        artifact would reach main unnoticed");
                }
                println!(
                    "  (declared workflows, not live run status — dash makes no network calls)"
                );
            }
        }

        if want("graph") {
            heading("Graph");
            if self.graph.pipelines.is_empty() {
                field("state", "no pipelines declared");
            } else {
                for p in &self.graph.pipelines {
                    println!("  {} ({})", p.name, p.executor);
                    for out in &p.outputs {
                        println!("    → {out}");
                    }
                }
            }
        }

        if want("freshness") {
            heading("Freshness");
            let f = &self.freshness;
            field(
                "artifacts",
                format!(
                    "{} fresh, {} stale, {} missing, {} never derived",
                    f.artifacts_fresh,
                    f.artifacts_stale,
                    f.artifacts_missing,
                    f.artifacts_untracked
                ),
            );
            field(
                "templates",
                if f.templates_modified == 0 {
                    "unmodified".to_string()
                } else {
                    format!("{} hand-edited since install", f.templates_modified)
                },
            );
            for p in &f.problems {
                println!("  ! {p}");
            }
            if f.problems.is_empty() {
                println!("  everything the lock tracks is current");
            }
        }

        println!();
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Render the workbench view for the product containing the current directory.
pub fn run(json: bool, section: Option<String>) -> Result<()> {
    if let Some(s) = &section {
        if !SECTIONS.contains(&s.as_str()) {
            anyhow::bail!(
                "unknown section `{s}` (expected one of: {})",
                SECTIONS.join(", ")
            );
        }
    }

    let cwd = std::env::current_dir()?;
    let root = Config::find_root(&cwd)?;
    let config = Config::load(&root.join(CONFIG_FILE))?;
    let dash = Dash::collect(&root, &config)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&dash)?);
    } else {
        dash.render(section.as_deref());
    }
    Ok(())
}
