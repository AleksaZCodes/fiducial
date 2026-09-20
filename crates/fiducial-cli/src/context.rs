//! Agent context, generated from the declarations it describes.
//!
//! Roadmap item **context sync**. The problem, as the roadmap states it:
//!
//! > `CLAUDE.md` and `AGENTS.md` both carried repository trees that had gone
//! > stale past three crates, *and both carried a disclaimer telling the reader
//! > to run `ls` instead of trusting them.* Phase 18 fixed them by hand and
//! > added a test. **A test that fails after the fact is detection, not sync.**
//!
//! So the derivable parts are derived. What an agent needs to know that is a
//! *fact about this repository* — which crates exist, which commands the binary
//! has, which capabilities ship, where the skills live — is generated from the
//! thing that decides it. What an agent needs to know that is *judgment* —
//! what not to do, why a rule exists, how to think about a capability — stays
//! hand-written, because nothing can derive it.
//!
//! # How a generated block is marked
//!
//! ```markdown
//! <!-- fid:begin crates -->
//! …generated…
//! <!-- fid:end crates -->
//! ```
//!
//! HTML comments, so they render as nothing on GitHub and in any Markdown
//! viewer. A file with no markers is left completely alone — this is opt-in per
//! file and per block, which is what lets a product adopt one block without
//! surrendering its whole `AGENTS.md`.
//!
//! # Why not `fid derive`
//!
//! `fid derive` runs a product's declared pipelines and records their outputs in
//! `fiducial.lock`. Agent context is not a product artifact: it exists in the
//! platform repository too, which deliberately declares no pipelines
//! (`fiducial.toml` says why — a gate that runs through the tool it gates is
//! blind exactly where it matters). `fid context --check` is the same contract
//! in its own command.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Context, Result};

/// A block the generator owns, by the name in its marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    /// The `crates/` and `packages/` tree, with each member's own description.
    Layout,
    /// Every `fid` command, from the binary's own definition.
    Commands,
    /// Every built-in capability, from the capability registry.
    Capabilities,
    /// Every adapter contract and the vendors selectable behind it.
    Adapters,
    /// Where the instructions an agent can load actually live.
    Skills,
}

impl Block {
    /// The marker name, e.g. `layout` in `<!-- fid:begin layout -->`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Layout => "layout",
            Self::Commands => "commands",
            Self::Capabilities => "capabilities",
            Self::Adapters => "adapters",
            Self::Skills => "skills",
        }
    }

    /// Every block this generator knows how to produce.
    pub const ALL: &'static [Block] = &[
        Block::Layout,
        Block::Commands,
        Block::Capabilities,
        Block::Adapters,
        Block::Skills,
    ];

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|b| b.name() == name)
    }
}

fn begin(name: &str) -> String {
    format!("<!-- fid:begin {name} -->")
}

fn end(name: &str) -> String {
    format!("<!-- fid:end {name} -->")
}

// ── Rendering each block ──────────────────────────────────────────────────────

/// The workspace tree, from the manifests that define it.
///
/// A member's one-line summary is its own `description` — the field crates.io
/// and npm already show. Writing a second summary into `AGENTS.md` is how the
/// tree went stale in the first place.
fn render_layout(root: &Path) -> Result<String> {
    let crates = workspace_members(root, "crates")?;
    let packages = workspace_members(root, "packages")?;

    let mut s = String::new();
    s.push_str("```\n");
    s.push_str("fiducial/\n");
    s.push_str(&format!(
        "├── crates/               {} members\n",
        crates.len()
    ));
    for (i, (name, desc)) in crates.iter().enumerate() {
        // The last child of `crates/` closes its own branch whether or not
        // `packages/` follows — that is a sibling, not a child.
        let branch = if i + 1 == crates.len() {
            "└──"
        } else {
            "├──"
        };
        s.push_str(&format!("│   {branch} {name:<24} {desc}\n"));
    }
    s.push_str(&format!(
        "└── packages/             {} members\n",
        packages.len()
    ));
    for (i, (name, desc)) in packages.iter().enumerate() {
        let branch = if i + 1 == packages.len() {
            "└──"
        } else {
            "├──"
        };
        s.push_str(&format!("    {branch} {name:<24} {desc}\n"));
    }
    s.push_str("```\n");
    Ok(s)
}

/// Read `<dir>/*/` and pull each member's name and description.
fn workspace_members(root: &Path, dir: &str) -> Result<Vec<(String, String)>> {
    let path = root.join(dir);
    if !path.is_dir() {
        return Ok(Vec::new());
    }
    let mut out: Vec<(String, String)> = Vec::new();
    for entry in std::fs::read_dir(&path)
        .with_context(|| format!("reading {dir}/"))?
        .flatten()
    {
        let member = entry.path();
        if !member.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(desc) = cargo_description(&member).or_else(|| npm_description(&member)) {
            out.push((name, desc));
        }
    }
    out.sort();
    Ok(out)
}

fn cargo_description(dir: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    let value: toml::Value = toml::from_str(&raw).ok()?;
    let desc = value.get("package")?.get("description")?.as_str()?;
    Some(one_line(desc))
}

fn npm_description(dir: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(dir.join("package.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    Some(one_line(value.get("description")?.as_str()?))
}

/// A description fit for a tree line: one line, and short enough to read.
fn one_line(raw: &str) -> String {
    let flat = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    // Cut on a character boundary — an em-dash straddling the limit panicked
    // `fid dash` once already, and these descriptions are full of them.
    if flat.chars().count() <= 62 {
        return flat;
    }
    let cut: String = flat.chars().take(59).collect();
    format!("{}…", cut.trim_end())
}

/// Every command the binary has, from the binary's own definition.
///
/// `clap` already holds this: the commands, their summaries and their
/// arguments are declared once, in `main.rs`, and a table in `AGENTS.md` is a
/// second copy that nothing keeps honest. The `--help` output and this table
/// now have one source.
fn render_commands(command: &clap::Command) -> String {
    let mut s = String::new();
    s.push_str("| Command | Does |\n|---|---|\n");
    for sub in command.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        let name = sub.get_name();
        let about = sub
            .get_about()
            .map(|a| a.to_string())
            .unwrap_or_else(|| String::from("—"));
        s.push_str(&format!("| `fid {name}` | {} |\n", escape_pipes(&about)));
    }
    s
}

/// Every built-in capability, and what it contributes.
fn render_capabilities() -> String {
    use crate::capability::Declaration;

    let mut s = String::new();
    s.push_str("| Capability | Contributes | Install |\n|---|---|---|\n");
    for cap in crate::capability::builtins() {
        let mut parts: Vec<String> = Vec::new();
        let decls: Vec<&str> = cap.declarations.iter().map(|d| d.name()).collect();
        if !decls.is_empty() {
            parts.push(format!("declares `{}`", decls.join("`, `")));
        }
        if !cap.pipelines.is_empty() {
            parts.push(format!("{} pipeline(s)", cap.pipelines.len()));
        }
        if !cap.requires_adapters.is_empty() {
            parts.push(format!("needs `{}`", cap.requires_adapters.join("`, `")));
        }
        if !cap.templates.is_empty() {
            parts.push(format!("{} template file(s)", cap.templates.len()));
        }
        if cap
            .declarations
            .iter()
            .any(|d| matches!(d, Declaration::ConfigBlock { .. }))
        {
            parts.push("seeds a `fiducial.toml` block".to_string());
        }
        if !cap.guard_rules.is_empty() {
            parts.push("guard rules".to_string());
        }
        if parts.is_empty() {
            parts.push("a skill".to_string());
        }
        s.push_str(&format!(
            "| `{}` | {} | `fid add capability {}` |\n",
            cap.id,
            escape_pipes(&parts.join("; ")),
            cap.id
        ));
    }
    s
}

/// Every adapter contract, and what can actually sit behind it today.
///
/// `AGENTS.md` carried this as prose — *"every contract currently implements
/// only `none`"* — which was true when it was written and false six vendors
/// later, once `d1`, `r2`, `cloudflare`, `turnstile`, `cloudflare-queues` and
/// `supabase` had landed behind it. The distinction that sentence was
/// protecting is the important one and is kept here: a vendor under
/// **Selectable today** works, and a vendor under **Planned** cannot be
/// selected at all. Stating it per contract, from the registry that decides
/// it, is what a sentence about all eight contracts could never stay right
/// about for more than one release.
fn render_adapters() -> String {
    let mut s = String::new();
    s.push_str("| Contract | For | Selectable today | Planned |\n|---|---|---|---|\n");
    for contract in crate::adapter::CONTRACTS {
        s.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            contract.name,
            escape_pipes(contract.description),
            vendor_list(contract.implementations),
            vendor_list(contract.candidates),
        ));
    }
    s
}

/// A contract's vendors as table cell text, or an em-dash when there are none.
fn vendor_list(vendors: &[&str]) -> String {
    if vendors.is_empty() {
        return "—".to_string();
    }
    vendors
        .iter()
        .map(|v| format!("`{v}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Where the instructions an agent can load actually live.
fn render_skills(root: &Path) -> Result<String> {
    let mut s = String::new();
    s.push_str("| File | Invoked as (Claude Code) | Does |\n|---|---|---|\n");

    let commands_dir = root.join("commands");
    if commands_dir.is_dir() {
        let mut found: BTreeMap<String, String> = BTreeMap::new();
        for entry in std::fs::read_dir(&commands_dir)
            .context("reading commands/")?
            .flatten()
        {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            found.insert(stem, front_matter_description(&text));
        }
        for (stem, desc) in found {
            s.push_str(&format!(
                "| `commands/{stem}.md` | `/fiducial:{stem}` | {} |\n",
                escape_pipes(&desc)
            ));
        }
    }

    s.push_str(
        "| `crates/fiducial-cli/capabilities/*/SKILL.md` | installed per capability \
         | How to use that capability |\n",
    );
    Ok(s)
}

/// A command file's `description:` front-matter, or its first line of prose.
fn front_matter_description(text: &str) -> String {
    let mut lines = text.lines();
    if lines.next().map(str::trim) == Some("---") {
        for line in lines.by_ref() {
            let t = line.trim();
            if t == "---" {
                break;
            }
            if let Some(rest) = t.strip_prefix("description:") {
                return one_line(rest.trim().trim_matches('"'));
            }
        }
    }
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("---"))
        .map(one_line)
        .unwrap_or_else(|| "—".to_string())
}

/// A table cell cannot contain a bare `|`.
fn escape_pipes(s: &str) -> String {
    s.replace('|', "\\|")
}

// ── Syncing a file ────────────────────────────────────────────────────────────

/// What one file's blocks look like after generation.
pub struct Rendered {
    /// The file, relative to the root.
    pub path: String,
    /// Its full content with every generated block refreshed.
    pub content: String,
    /// Blocks actually found in it.
    pub blocks: Vec<Block>,
    /// True when the file on disk already matches.
    pub fresh: bool,
}

/// Files whose generated blocks this command maintains.
///
/// A file with no markers is skipped, so this list is "where to look", not
/// "what must exist" — a product with no `CLAUDE.md` is not a finding.
pub const CONTEXT_FILES: &[&str] = &["AGENTS.md", "CLAUDE.md", "README.md"];

/// Regenerate every marked block in a file.
pub fn render_file(root: &Path, rel: &str, command: &clap::Command) -> Result<Option<Rendered>> {
    let path = root.join(rel);
    let Ok(original) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    if !original.contains("<!-- fid:begin ") {
        return Ok(None);
    }

    let mut content = original.clone();
    let mut blocks = Vec::new();

    // Every marker present, not every block known: a file opts in per block.
    for block in Block::ALL {
        let (b, e) = (begin(block.name()), end(block.name()));
        if !content.contains(&b) {
            continue;
        }
        let body = match block {
            Block::Layout => render_layout(root)?,
            Block::Commands => render_commands(command),
            Block::Capabilities => render_capabilities(),
            Block::Adapters => render_adapters(),
            Block::Skills => render_skills(root)?,
        };
        content = replace_block(&content, &b, &e, &body, rel)?;
        blocks.push(*block);
    }

    // A marker this generator does not know is a silent no-op otherwise: the
    // block stays frozen at whatever it said, looking generated and being hand
    // written, which is the worst of both.
    let unknown = unknown_markers(&content);
    if !unknown.is_empty() {
        bail!(
            "{rel} marks block(s) `{}` that `fid context` cannot generate.\n\
             Known blocks: {}",
            unknown.join("`, `"),
            Block::ALL
                .iter()
                .map(|b| b.name())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok(Some(Rendered {
        path: rel.to_string(),
        fresh: content == original,
        content,
        blocks,
    }))
}

fn replace_block(text: &str, begin: &str, end: &str, body: &str, rel: &str) -> Result<String> {
    let Some(start) = text.find(begin) else {
        return Ok(text.to_string());
    };
    let after = start + begin.len();
    let Some(stop) = text[after..].find(end) else {
        bail!("{rel}: `{begin}` has no matching `{end}`");
    };
    let stop = after + stop;
    Ok(format!(
        "{}{begin}\n{}{}",
        &text[..start],
        body,
        &text[stop..]
    ))
}

/// Markers in a file that name a block this generator does not know.
fn unknown_markers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("<!-- fid:begin ") else {
            continue;
        };
        let name = rest.trim_end_matches("-->").trim();
        if Block::from_name(name).is_none() {
            out.push(name.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every command has a description, and no command has two.
    ///
    /// Both halves of that came from one real bug, reported by an outside
    /// reader: the `///` and `long_about` written for `Docs` were placed above
    /// `Design` instead, so `clap` concatenated two summaries into `fid design`
    /// and left `fid docs` with none. It reached `main` and sat there, because
    /// nothing looked. `render_commands` printed `—` for the empty one — a
    /// dash in a committed table is not loud enough to notice.
    ///
    /// Asserting on `get_about()` rather than on the generated table keeps this
    /// pointed at the cause: the table is derived, so a test against its text
    /// would fail for formatting reasons too and stop meaning this.
    ///
    /// The upper bound catches the *other* direction. A missing description is
    /// invisible; a doubled one reads as prose and is easy to skim past, which
    /// is exactly what happened. Two concatenated summaries comfortably exceed
    /// any single one written here.
    #[test]
    fn every_command_has_exactly_one_description() {
        use clap::CommandFactory;

        const MAX: usize = 90;
        let command = crate::Cli::command();

        let mut missing: Vec<&str> = Vec::new();
        let mut doubled: Vec<String> = Vec::new();

        for sub in command.get_subcommands() {
            if sub.is_hide_set() {
                continue;
            }
            match sub.get_about() {
                None => missing.push(sub.get_name()),
                Some(about) => {
                    let text = about.to_string();
                    if text.chars().count() > MAX {
                        doubled.push(format!(
                            "  fid {} — {} chars: {text}",
                            sub.get_name(),
                            text.chars().count()
                        ));
                    }
                }
            }
        }

        assert!(
            missing.is_empty(),
            "these commands have no `///` summary, so `fid <cmd> --help` and the \
             AGENTS.md table both show nothing for them:\n\n  {}\n\n\
             The usual cause is the summary being attached to the variant above.",
            missing.join("\n  ")
        );
        assert!(
            doubled.is_empty(),
            "these command summaries are over {MAX} characters, which usually means \
             two `///` blocks stacked on one variant:\n\n{}\n\n\
             Check whether the first belongs to the variant above it.",
            doubled.join("\n")
        );
    }

    #[test]
    fn a_description_is_shortened_on_a_character_boundary() {
        // These descriptions are full of em-dashes; `truncate` panics on one.
        let long = "no_std frame codec — the waist of the hourglass, mirrored in \
                    TypeScript and pinned by conformance vectors";
        let out = one_line(long);
        assert!(out.chars().count() <= 63, "{out}");
        assert!(out.ends_with('…'));
    }

    #[test]
    fn a_multi_line_description_becomes_one_line() {
        assert_eq!(one_line("a\n  b   c"), "a b c");
    }

    #[test]
    fn a_pipe_cannot_break_a_table_row() {
        assert_eq!(escape_pipes("a | b"), "a \\| b");
    }

    #[test]
    fn a_block_is_replaced_between_its_markers_and_nothing_else_moves() {
        let text = "before\n<!-- fid:begin layout -->\nOLD\n<!-- fid:end layout -->\nafter\n";
        let out =
            replace_block(text, &begin("layout"), &end("layout"), "NEW\n", "AGENTS.md").unwrap();
        assert_eq!(
            out,
            "before\n<!-- fid:begin layout -->\nNEW\n<!-- fid:end layout -->\nafter\n"
        );
    }

    #[test]
    fn an_unclosed_block_is_an_error_rather_than_a_truncated_file() {
        let text = "<!-- fid:begin layout -->\nOLD\n";
        let err = replace_block(text, &begin("layout"), &end("layout"), "NEW\n", "AGENTS.md")
            .unwrap_err();
        assert!(format!("{err:#}").contains("no matching"), "{err:#}");
    }

    #[test]
    fn every_contract_is_rendered_with_none_always_selectable() {
        let table = render_adapters();
        for contract in crate::adapter::CONTRACTS {
            assert!(
                table.contains(&format!("| `{}` |", contract.name)),
                "contract `{}` missing from the generated table",
                contract.name
            );
        }
        // `none` is a real implementation, not a placeholder — if it ever stops
        // being selectable on every contract, that is a platform change and
        // this table is how a reader would find out.
        let rows = table.lines().filter(|l| l.starts_with("| `")).count();
        assert_eq!(rows, crate::adapter::CONTRACTS.len());
    }

    #[test]
    fn a_contract_with_no_candidates_renders_a_dash_rather_than_an_empty_cell() {
        assert_eq!(vendor_list(&[]), "—");
        assert_eq!(vendor_list(&["none", "d1"]), "`none`, `d1`");
    }

    #[test]
    fn a_marker_naming_an_unknown_block_is_found() {
        let found = unknown_markers("<!-- fid:begin layout -->\n<!-- fid:begin weather -->\n");
        assert_eq!(found, vec!["weather"]);
    }

    #[test]
    fn front_matter_description_is_preferred_over_prose() {
        let text =
            "---\nname: harvest\ndescription: Extract reusable work\n---\n\n# Harvest\n\nProse.\n";
        assert_eq!(front_matter_description(text), "Extract reusable work");
    }

    #[test]
    fn without_front_matter_the_first_line_of_prose_is_used() {
        assert_eq!(
            front_matter_description("# Title\n\nWhat this does.\n"),
            "What this does."
        );
    }
}
