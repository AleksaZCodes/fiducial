//! `fid thesis` — declare, read and sharpen the one claim a product tests.
//!
//! | Command | What it does |
//! |---|---|
//! | `fid thesis` | Print the current thesis and what is still unanswered |
//! | `fid thesis log` | Print every thesis this product has held, and why each moved |
//! | `fid thesis set "<claim>"` | Append a thesis, superseding the current one |
//!
//! # What this command will not do
//!
//! It will not edit an existing entry. `thesis.toml` is append-only, so `set`
//! only ever appends — and when a thesis is already in force, the new entry
//! supersedes it and must say `because`. Sharpening a claim is the interesting
//! event in a product's life; overwriting the old wording destroys the only
//! record of it.
//!
//! It will not fill in the other fields. `set` takes a claim and nothing else,
//! because the arc, the test and the evidence are written by thinking, in the
//! file, not by remembering nine flag names. `fid thesis` prints the question
//! each empty field answers, and `fid advise` argues with the answers.
//!
//! It will not stop you shipping. Nothing here is a `fid derive --check` gate.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use std::{fs, path::Path};

use crate::thesis::{self, THESIS_FILE};

#[derive(Subcommand)]
pub enum ThesisAction {
    /// Print every thesis this product has held, oldest first
    #[command(long_about = "\
Print the full history: every thesis, in file order, with the reason each one
replaced the last.

This is the view that shows how the thinking matured — which is the thing a
prose MISSION.md cannot show, because editing a paragraph destroys what it
said before.")]
    Log,

    /// Append a thesis, superseding the current one
    #[command(long_about = "\
Append a thesis to thesis.toml.

The first one needs only a claim:

  fid thesis set \"No unverified alert ever reaches a responder.\"

Once a thesis is in force, a new one replaces it and must say why:

  fid thesis set \"<sharper claim>\" --because \"Cheap was never the objection.\"

Nothing is edited. The old entry stays exactly as it was written, marked
superseded, which is how `fid thesis log` can show the evolution at all.

The other fields — the arc, the test, the evidence — are written by hand in
thesis.toml. Run `fid thesis` to see which are still empty and what each one
asks.")]
    Set {
        /// The claim. One sentence, the one you would sell with.
        #[arg(value_name = "CLAIM")]
        claim: String,

        /// Why the thesis moved. Required once a thesis is already in force.
        #[arg(long, value_name = "TEXT")]
        because: Option<String>,

        /// Date to record, `YYYY-MM-DD`. Defaults to today.
        #[arg(long, value_name = "DATE")]
        date: Option<String>,
    },
}

pub fn run(action: Option<ThesisAction>, json: bool) -> Result<()> {
    let root = std::env::current_dir().context("reading the working directory")?;
    if json {
        return emit_json(&root);
    }
    match action {
        None => show(&root),
        Some(ThesisAction::Log) => log(&root),
        Some(ThesisAction::Set {
            claim,
            because,
            date,
        }) => set(&root, &claim, because.as_deref(), date.as_deref()),
    }
}

// ── show ──────────────────────────────────────────────────────────────────────

fn show(root: &Path) -> Result<()> {
    let file = load(root)?;
    let t = thesis::current(&file)?;

    println!("✦ fid thesis — {}", root.display());
    println!();
    println!("  {}", t.claim.trim());
    println!();
    println!("  declared {}", t.date);
    if let Some(b) = &t.because {
        println!("  because  {}", b.trim());
    }

    let gaps = thesis::gaps(t);
    println!();
    if gaps.is_empty() {
        println!("  ✓ every part of this thesis is stated.");
        println!();
        println!("  That is not the same as it being right. `fid advise` argues with it.");
    } else {
        let stated = 11 - gaps.len();
        println!("  {stated}/11 parts stated. Still unanswered:");
        println!();
        for g in &gaps {
            println!("    {:<20} {}", g.field, g.question);
        }
        println!();
        println!("  Answer them in {THESIS_FILE}, in your own words, whenever they become");
        println!("  answerable. None of this blocks anything.");
    }
    println!();
    Ok(())
}

// ── json ──────────────────────────────────────────────────────────────────────

/// The machine-readable view: what is claimed, what is unanswered, what it
/// replaced.
///
/// This exists so nothing else has to parse `thesis.toml`. `fid advise` is
/// JavaScript and would otherwise need a TOML reader plus its own idea of which
/// entry is current and which fields are missing — a second parser for one
/// declaration, which is the duplication this whole platform is against.
///
/// A product with no thesis yet emits `{"declared": false}` and exits 0. It is
/// a normal state, and a consumer that has to distinguish "no thesis" from "the
/// command failed" by reading stderr will get it wrong.
fn emit_json(root: &Path) -> Result<()> {
    let path = root.join(THESIS_FILE);
    let file = match fs::read_to_string(&path) {
        Ok(raw) => thesis::parse(&raw)?,
        Err(_) => thesis::ThesisFile::default(),
    };

    if file.thesis.is_empty() {
        println!("{}", serde_json::json!({ "declared": false }));
        return Ok(());
    }

    let t = thesis::current(&file)?;
    let gaps: Vec<serde_json::Value> = thesis::gaps(t)
        .iter()
        .map(|g| serde_json::json!({ "field": g.field, "question": g.question }))
        .collect();

    let out = serde_json::json!({
        "declared": true,
        "current": t,
        "gaps": gaps,
        "history": file.thesis,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

// ── log ───────────────────────────────────────────────────────────────────────

fn log(root: &Path) -> Result<()> {
    let file = load(root)?;
    if file.thesis.is_empty() {
        bail!("{THESIS_FILE} declares no thesis yet. Add one with `fid thesis set \"<claim>\"`.");
    }

    let superseded: Vec<usize> = file.thesis.iter().filter_map(|t| t.supersedes).collect();

    println!("✦ fid thesis log — {}", root.display());
    println!();
    for (i, t) in file.thesis.iter().enumerate() {
        let n = i + 1;
        let mark = if superseded.contains(&n) {
            "superseded"
        } else {
            "current"
        };
        println!("  {n}. {}  [{mark}]", t.date);
        println!("     {}", t.claim.trim());
        if let Some(b) = &t.because {
            println!("     ↑ {}", b.trim());
        }
        println!();
    }
    Ok(())
}

// ── set ───────────────────────────────────────────────────────────────────────

fn set(root: &Path, claim: &str, because: Option<&str>, date: Option<&str>) -> Result<()> {
    if claim.trim().is_empty() {
        bail!("a thesis needs a claim.");
    }

    let path = root.join(THESIS_FILE);
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let file = thesis::parse(&existing)?;
    let count = file.thesis.len();

    // The index of the entry this replaces: the current one, if there is one.
    let supersedes = if count == 0 {
        None
    } else {
        let superseded: Vec<usize> = file.thesis.iter().filter_map(|t| t.supersedes).collect();
        (1..=count).rev().find(|n| !superseded.contains(n))
    };

    if supersedes.is_some() && because.is_none() {
        bail!(
            "this product already holds a thesis, so a new one replaces it and has to say \
             why.\n\n  fid thesis set \"{}\" --because \"<what changed in your thinking>\"\n\n\
             Run `fid thesis log` to see the one you are about to supersede.",
            claim.trim()
        );
    }

    let date = date.map(str::to_string).unwrap_or_else(today_iso8601);

    let mut entry = String::new();
    if !existing.is_empty() && !existing.ends_with('\n') {
        entry.push('\n');
    }
    if count > 0 {
        entry.push('\n');
    }
    entry.push_str("[[thesis]]\n");
    entry.push_str(&format!("date  = {}\n", toml_str(&date)));
    entry.push_str(&format!("claim = {}\n", toml_str(claim.trim())));
    if let Some(s) = supersedes {
        entry.push_str(&format!("supersedes = {s}\n"));
    }
    if let Some(b) = because {
        entry.push_str(&format!("because    = {}\n", toml_str(b.trim())));
    }

    let updated = format!("{existing}{entry}");
    // Parse what is about to be written, not what was intended. A file this
    // command cannot read back is a file the product is now stuck with.
    thesis::parse(&updated).context("the appended thesis would not parse — nothing was written")?;
    fs::write(&path, &updated).with_context(|| format!("writing {}", path.display()))?;

    println!("✦ fid thesis set");
    println!();
    println!("  {}", claim.trim());
    println!();
    match supersedes {
        Some(s) => println!(
            "  Appended to {THESIS_FILE} as thesis {}, superseding {s}.",
            count + 1
        ),
        None => println!("  Appended to {THESIS_FILE} as this product's first thesis."),
    }
    println!("  Run `fid thesis` to see what is still unanswered.");
    println!();
    Ok(())
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn load(root: &Path) -> Result<thesis::ThesisFile> {
    let path = root.join(THESIS_FILE);
    let raw = fs::read_to_string(&path).with_context(|| {
        format!(
            "reading {}. Declare a thesis with `fid thesis set \"<claim>\"`.",
            path.display()
        )
    })?;
    thesis::parse(&raw)
}

/// A TOML basic string.
///
/// Shares the escape set with `thesis::render_ts` for the same reason: this is
/// the inverse of parsing, and a string this command writes must be one the
/// parser reads back identically.
fn toml_str(v: &str) -> String {
    let mut out = String::with_capacity(v.len() + 2);
    out.push('"');
    for c in v.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Today as `YYYY-MM-DD`, the same way `fid release` records a history entry.
fn today_iso8601() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn the_first_thesis_needs_no_reason() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        set(root, "Ship it.", None, Some("2026-09-21")).unwrap();
        let raw = fs::read_to_string(root.join(THESIS_FILE)).unwrap();
        assert!(raw.contains("claim = \"Ship it.\""), "{raw}");
        assert!(!raw.contains("supersedes"), "{raw}");
    }

    #[test]
    fn replacing_a_thesis_without_a_reason_is_refused() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        set(root, "First.", None, Some("2026-09-21")).unwrap();
        let err = set(root, "Second.", None, Some("2026-09-22")).unwrap_err();
        assert!(err.to_string().contains("has to say why"), "{err}");

        // And the refusal changed nothing.
        let raw = fs::read_to_string(root.join(THESIS_FILE)).unwrap();
        assert!(!raw.contains("Second."), "{raw}");
    }

    #[test]
    fn replacing_a_thesis_appends_and_leaves_the_old_wording_intact() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        set(root, "First.", None, Some("2026-09-21")).unwrap();
        set(root, "Second.", Some("It got sharper."), Some("2026-09-22")).unwrap();

        let raw = fs::read_to_string(root.join(THESIS_FILE)).unwrap();
        assert!(raw.contains("claim = \"First.\""), "the old entry survives");
        assert!(raw.contains("supersedes = 1"), "{raw}");

        let file = thesis::parse(&raw).unwrap();
        assert_eq!(file.thesis.len(), 2);
        assert_eq!(thesis::current(&file).unwrap().claim, "Second.");
    }

    #[test]
    fn a_third_thesis_supersedes_the_second_not_the_first() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        set(root, "One.", None, Some("2026-01-01")).unwrap();
        set(root, "Two.", Some("a"), Some("2026-02-01")).unwrap();
        set(root, "Three.", Some("b"), Some("2026-03-01")).unwrap();

        let file = thesis::parse(&fs::read_to_string(root.join(THESIS_FILE)).unwrap()).unwrap();
        assert_eq!(file.thesis[2].supersedes, Some(2));
        assert_eq!(thesis::current(&file).unwrap().claim, "Three.");
    }

    #[test]
    fn a_quoted_claim_round_trips() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        set(root, r#"They said "never"."#, None, Some("2026-09-21")).unwrap();
        let file = thesis::parse(&fs::read_to_string(root.join(THESIS_FILE)).unwrap()).unwrap();
        assert_eq!(
            thesis::current(&file).unwrap().claim,
            r#"They said "never"."#
        );
    }

    #[test]
    fn an_empty_claim_is_refused() {
        let dir = TempDir::new().unwrap();
        assert!(set(dir.path(), "   ", None, None).is_err());
    }

    #[test]
    fn the_recorded_date_is_a_plain_iso_day() {
        let today = today_iso8601();
        assert_eq!(today.len(), 10, "{today}");
        assert!(today.chars().filter(|c| *c == '-').count() == 2, "{today}");
    }
}
