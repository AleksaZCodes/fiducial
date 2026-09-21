//! The thesis — one claim a product is built to test, declared once.
//!
//! # Why this exists
//!
//! `MISSION.md` is prose. fon's is thirty-three lines, and the sentence that
//! actually decides things — *"no unverified alert ever reaches a responder"* —
//! is in the middle of its second paragraph. It is a good thesis: falsifiable,
//! it states a tension, and it is the sentence you would sell with. But nothing
//! downstream can find it. `messages/en.json` restates it by hand as "A person
//! confirms every event"; `README.md` states it a third way; an editor judging a
//! draft has no canonical wording to judge against. Three copies, no source.
//!
//! That is a **structure** problem, not a writing problem. The claim was already
//! sharp. It was unaddressable.
//!
//! So a thesis is declared the way every other fact in this platform is
//! declared: once, structured, in a file, with its parts separable so each can
//! be interrogated on its own.
//!
//! # A Fact with a Decision's lifecycle
//!
//! A thesis is a Fact — hand-authored, never generated, and things derive from
//! it. But unlike `[brand] primary_color` it is not *corrected*, it is
//! **sharpened**, and how it sharpened is most of what a reader wants. So it
//! carries a Decision's lifecycle: `thesis.toml` is append-only, a new entry
//! supersedes an older one and says `because`, and nothing is ever edited in
//! place.
//!
//! That pairing is deliberate and is the only place in the platform where the
//! two lifecycles meet. `docs/specs/2026-09-21-the-thesis-is-a-fact.md` argues
//! it.
//!
//! # One line is a valid thesis
//!
//! `claim` is the only required field. Everything else — the arc, the test, the
//! evidence — is optional and fillable months later. A thesis you cannot state
//! completely on day one is the normal case, and a tool that refuses the rough
//! version is a tool you route around. `fid advise` asks after the empty fields;
//! it never blocks, and nothing here is a `fid derive --check` gate.
//!
//! # Pure by construction
//!
//! Everything in this module is a function of the parsed declaration. The I/O —
//! reading `thesis.toml`, writing `PITCH.md` — belongs to `commands/thesis.rs`
//! and the `fid-thesis` executor in `commands/derive.rs`, the same split
//! `brand.rs` and `i18n.rs` use.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// The file a product declares its thesis in.
pub const THESIS_FILE: &str = "thesis.toml";

/// The whole declaration: every thesis this product has held, oldest first.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ThesisFile {
    #[serde(default)]
    pub thesis: Vec<Thesis>,
}

/// One thesis, as held on one date.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Thesis {
    /// When this thesis was adopted, `YYYY-MM-DD`.
    pub date: String,

    /// The claim. The sentence you sell with, and the only required field.
    pub claim: String,

    /// 1-based index of the entry this replaces, in file order.
    ///
    /// An index rather than a date or a slug: the file *is* the history, so the
    /// position in it is the only identity an entry needs, and it cannot be
    /// mistyped into pointing at nothing without `validate` noticing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<usize>,

    /// Why the thesis moved. Required whenever `supersedes` is set.
    ///
    /// This is the field that makes the history worth keeping. A superseding
    /// entry without it records *that* the thinking changed and destroys *how*,
    /// which is the part a reader cannot reconstruct.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub because: Option<String>,

    /// The narrative: what you would walk someone through.
    #[serde(default, skip_serializing_if = "Arc::is_empty")]
    pub arc: Arc,

    /// What makes it a thesis rather than a slogan.
    #[serde(default, skip_serializing_if = "Test::is_empty")]
    pub test: Test,

    /// What may honestly be claimed today.
    #[serde(default, skip_serializing_if = "Evidence::is_empty")]
    pub evidence: Evidence,
}

/// The pitch, in the order you would say it out loud.
///
/// The field order is the mission-statement formula — what, for whom, against
/// what problem, why now, why us, how, where — and it is the order `PITCH.md`
/// renders in. Ordering the struct to match means the rendered pitch and the
/// declaration read the same way, so there is no second place that decides how
/// a pitch is sequenced.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Arc {
    /// What are we making?
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub what: Option<String>,
    /// Who is it for?
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub for_whom: Option<String>,
    /// The tension it resolves. Not a description — a conflict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    /// What changed that makes this possible or urgent now?
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why_now: Option<String>,
    /// Why this team — the lineage, the unfair advantage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why_us: Option<String>,
    /// The mechanism. The smallest thing that would test the claim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub how: Option<String>,
    /// Where it happens — the geography, channel or context.
    ///
    /// `where` is a Rust keyword, so the field carries the trailing underscore
    /// and the rename keeps the declaration readable. A product writes
    /// `where = "Serbia."`, which is the only spelling anyone should have to
    /// know.
    #[serde(rename = "where", default, skip_serializing_if = "Option::is_none")]
    pub where_: Option<String>,
}

/// The falsifiability test. A thesis nobody could disagree with is worthless.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Test {
    /// What observation would prove the claim wrong?
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub falsified_by: Option<String>,
    /// Who is the reasonable person who disagrees, and what do they hold?
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disagrees: Option<String>,
}

/// What is actually true today, separated from what is intended.
///
/// This group exists because fon's `MISSION.md` has to end with a paragraph
/// saying no deployment, pilot or live alerting exists — in prose, where no
/// generator can see it. A thesis that derives pitch copy and cannot say what
/// is unproven will produce a pitch that overclaims, and it will do it
/// silently.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Evidence {
    /// What you have actually shown.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proven: Vec<String>,
    /// What you have not, stated plainly enough to publish.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_yet: Vec<String>,
}

impl Arc {
    pub fn is_empty(&self) -> bool {
        self.what.is_none()
            && self.for_whom.is_none()
            && self.problem.is_none()
            && self.why_now.is_none()
            && self.why_us.is_none()
            && self.how.is_none()
            && self.where_.is_none()
    }
}

impl Test {
    pub fn is_empty(&self) -> bool {
        self.falsified_by.is_none() && self.disagrees.is_none()
    }
}

impl Evidence {
    pub fn is_empty(&self) -> bool {
        self.proven.is_empty() && self.not_yet.is_empty()
    }
}

// ── parsing and resolution ────────────────────────────────────────────────────

/// Parse and validate a `thesis.toml`.
///
/// Parsing and validation are one call on purpose: a declaration that parses
/// but contradicts itself — superseding an entry that does not exist, say — is
/// not a thesis anyone should be handed, and every caller here wants both.
pub fn parse(raw: &str) -> Result<ThesisFile> {
    let file: ThesisFile = toml::from_str(raw).context("parsing thesis.toml")?;
    validate(&file)?;
    Ok(file)
}

/// Check the declaration for the three ways it can be internally inconsistent.
///
/// Note what is *not* checked: whether the claim is any good. That judgment
/// varies by reader and by day, so it belongs to `fid advise`, never to a gate
/// (`docs/specs/2026-09-20-two-kinds-of-model.md`).
pub fn validate(file: &ThesisFile) -> Result<()> {
    for (i, t) in file.thesis.iter().enumerate() {
        let n = i + 1;
        if t.claim.trim().is_empty() {
            bail!("thesis {n} has no claim. The claim is the one required field.");
        }
        if t.date.trim().is_empty() {
            bail!("thesis {n} has no date. Use YYYY-MM-DD.");
        }
        match t.supersedes {
            Some(0) => bail!("thesis {n} supersedes 0; entries are numbered from 1."),
            Some(s) if s > file.thesis.len() => bail!(
                "thesis {n} supersedes {s}, but there are only {} entries.",
                file.thesis.len()
            ),
            Some(s) if s >= n => bail!(
                "thesis {n} supersedes {s}, which is not an earlier entry. \
                 A thesis can only replace one that came before it."
            ),
            Some(_) if t.because.is_none() => bail!(
                "thesis {n} supersedes an earlier one without saying `because`. \
                 Recording that the thinking changed while dropping how it changed \
                 is the part a reader cannot reconstruct."
            ),
            _ => {}
        }
    }
    Ok(())
}

/// The thesis currently in force: the last entry nothing supersedes.
///
/// "Last" rather than "the only one" — an entry may be superseded by a later
/// entry that is itself superseded, and the file is the history, so the answer
/// is positional.
pub fn current(file: &ThesisFile) -> Result<&Thesis> {
    let superseded: Vec<usize> = file.thesis.iter().filter_map(|t| t.supersedes).collect();
    file.thesis
        .iter()
        .enumerate()
        .rev()
        .find(|(i, _)| !superseded.contains(&(i + 1)))
        .map(|(_, t)| t)
        .context("thesis.toml declares no thesis yet. Add one with `fid thesis set \"<claim>\"`.")
}

// ── completeness ──────────────────────────────────────────────────────────────

/// A field a thesis has not filled in yet, and the question that would fill it.
///
/// Returned rather than printed so `fid thesis`, `fid doctor` and the advisor
/// all ask the same questions in the same words. The prompts are the actual
/// sharpening tool; leaving each caller to phrase its own would make the
/// product's thesis depend on which command you happened to run.
pub struct Gap {
    pub field: &'static str,
    pub question: &'static str,
}

/// Every unfilled part of a thesis, in the order worth filling them.
///
/// Falsifiability leads. A claim nobody could disagree with is the one failure
/// that makes the rest of the exercise pointless, so it is the first thing
/// asked — before the narrative, which is easy and comfortable to write.
pub fn gaps(t: &Thesis) -> Vec<Gap> {
    let mut out = Vec::new();
    let mut ask = |present: bool, field, question| {
        if !present {
            out.push(Gap { field, question });
        }
    };

    ask(
        t.test.falsified_by.is_some(),
        "test.falsified_by",
        "What would you have to observe to admit this claim is wrong?",
    );
    ask(
        t.test.disagrees.is_some(),
        "test.disagrees",
        "Who is the reasonable person who disagrees, and what do they hold instead?",
    );
    ask(
        t.arc.problem.is_some(),
        "arc.problem",
        "What tension does this resolve? Name the conflict, not the topic.",
    );
    ask(t.arc.what.is_some(), "arc.what", "What are you making?");
    ask(
        t.arc.for_whom.is_some(),
        "arc.for_whom",
        "Who is it for? Name them specifically enough to go and find one.",
    );
    ask(
        t.arc.why_now.is_some(),
        "arc.why_now",
        "What changed that makes this possible or urgent now, and not five years ago?",
    );
    ask(
        t.arc.why_us.is_some(),
        "arc.why_us",
        "Why you? What do you carry that someone starting today does not?",
    );
    ask(
        t.arc.how.is_some(),
        "arc.how",
        "How, mechanically? The smallest thing that would actually test the claim.",
    );
    ask(
        t.arc.where_.is_some(),
        "arc.where",
        "Where does this happen — which geography, channel or context?",
    );
    ask(
        !t.evidence.proven.is_empty(),
        "evidence.proven",
        "What have you actually shown so far?",
    );
    ask(
        !t.evidence.not_yet.is_empty(),
        "evidence.not_yet",
        "What have you NOT shown? This is what stops derived copy from overclaiming.",
    );
    out
}

// ── rendering ─────────────────────────────────────────────────────────────────

/// `PITCH.md` — the arc assembled in the order you would say it.
///
/// Sections appear only when declared. A pitch with holes in it reads as a
/// pitch you are still working out, which is honest; a pitch with
/// "_(not yet stated)_" under seven headings reads as a form, and nobody sends
/// a form to an evaluator.
pub fn render_pitch_md(t: &Thesis, product_name: &str, generator: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("# {product_name}\n\n"));
    s.push_str(&format!("> **{}**\n\n", t.claim.trim()));

    let mut section = |heading: &str, body: &Option<String>| {
        if let Some(v) = body {
            s.push_str(&format!("## {heading}\n\n{}\n\n", v.trim()));
        }
    };

    section("What this is", &t.arc.what);
    section("Who it is for", &t.arc.for_whom);
    section("The problem", &t.arc.problem);
    section("Why now", &t.arc.why_now);
    section("Why us", &t.arc.why_us);
    section("How it works", &t.arc.how);
    section("Where", &t.arc.where_);

    if !t.test.is_empty() {
        s.push_str("## What would prove this wrong\n\n");
        if let Some(f) = &t.test.falsified_by {
            s.push_str(&format!("{}\n\n", f.trim()));
        }
        if let Some(d) = &t.test.disagrees {
            s.push_str(&format!("**Who disagrees.** {}\n\n", d.trim()));
        }
    }

    if !t.evidence.is_empty() {
        s.push_str("## Where this actually stands\n\n");
        for p in &t.evidence.proven {
            s.push_str(&format!("- ✓ {}\n", p.trim()));
        }
        for n in &t.evidence.not_yet {
            s.push_str(&format!("- ✗ {}\n", n.trim()));
        }
        s.push('\n');
    }

    s.push_str("---\n\n");
    s.push_str(&format!(
        "_Derived from `thesis.toml` by `{generator}`, thesis of {}. \
         Do not edit — edit the thesis._\n",
        t.date
    ));
    s
}

/// `thesis.ts` — the same declaration, addressable from application code.
///
/// The point of this file is that a landing page, a metadata export or an
/// editorial check imports the claim instead of retyping it. `as const` so a
/// consumer gets the literal string in its type and a typo in a comparison is
/// a compile error rather than a silent false.
pub fn render_ts(t: &Thesis, generator: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "// Generated by `{generator}` from thesis.toml — do not edit.\n\
         // Edit the thesis; this file is derived from it.\n\n"
    ));
    s.push_str("export const THESIS = {\n");
    s.push_str(&format!("  date: {},\n", json_str(&t.date)));
    s.push_str(&format!("  claim: {},\n", json_str(&t.claim)));

    let mut field = |name: &str, v: &Option<String>| {
        if let Some(v) = v {
            s.push_str(&format!("  {name}: {},\n", json_str(v)));
        }
    };
    field("what", &t.arc.what);
    field("forWhom", &t.arc.for_whom);
    field("problem", &t.arc.problem);
    field("whyNow", &t.arc.why_now);
    field("whyUs", &t.arc.why_us);
    field("how", &t.arc.how);
    field("where", &t.arc.where_);
    field("falsifiedBy", &t.test.falsified_by);
    field("disagrees", &t.test.disagrees);

    if !t.evidence.proven.is_empty() {
        s.push_str(&format!("  proven: {},\n", json_list(&t.evidence.proven)));
    }
    if !t.evidence.not_yet.is_empty() {
        s.push_str(&format!("  notYet: {},\n", json_list(&t.evidence.not_yet)));
    }

    s.push_str("} as const;\n");
    s
}

/// A TOML string as a JSON/TS string literal.
///
/// Hand-rolled rather than pulled from `serde_json`: the whole output of this
/// module has to be byte-stable across versions of everything, and an escape
/// this narrow is cheaper to pin here than to depend on.
fn json_str(v: &str) -> String {
    let mut out = String::with_capacity(v.len() + 2);
    out.push('"');
    for c in v.trim().chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push(' '),
            '\r' => {}
            '\t' => out.push(' '),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_list(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|i| json_str(i)).collect();
    format!("[{}]", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FON: &str = r#"
[[thesis]]
date  = "2026-08-02"
claim = "Wildfire detection should be cheap and solar."

[[thesis]]
date       = "2026-09-21"
claim      = "No unverified alert ever reaches a responder."
supersedes = 1
because    = "Cheap was never the objection. Trust was."

[thesis.arc]
what     = "A solar LoRa mesh sensor network."
for_whom = "Municipalities, national parks, public forest enterprises."
problem  = "A network that cries wolf is worse than no network."
where    = "Serbia."

[thesis.test]
falsified_by = "Responders act on machine-only alerts at the same rate."

[thesis.evidence]
not_yet = ["No field deployment. No pilot. No live public alerting."]
"#;

    #[test]
    fn one_line_is_a_valid_thesis() {
        let f = parse("[[thesis]]\ndate = \"2026-09-21\"\nclaim = \"Ship it.\"\n").unwrap();
        assert_eq!(current(&f).unwrap().claim, "Ship it.");
    }

    #[test]
    fn the_current_thesis_is_the_one_nothing_supersedes() {
        let f = parse(FON).unwrap();
        assert_eq!(
            current(&f).unwrap().claim,
            "No unverified alert ever reaches a responder."
        );
    }

    #[test]
    fn where_is_readable_despite_being_a_rust_keyword() {
        let f = parse(FON).unwrap();
        assert_eq!(current(&f).unwrap().arc.where_.as_deref(), Some("Serbia."));
    }

    #[test]
    fn a_claimless_thesis_is_rejected() {
        let err = parse("[[thesis]]\ndate = \"2026-09-21\"\nclaim = \"  \"\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("no claim"), "{err}");
    }

    #[test]
    fn superseding_without_a_reason_is_rejected() {
        let raw = "[[thesis]]\ndate=\"2026-01-01\"\nclaim=\"A\"\n\n\
                   [[thesis]]\ndate=\"2026-02-01\"\nclaim=\"B\"\nsupersedes=1\n";
        let err = parse(raw).unwrap_err().to_string();
        assert!(err.contains("because"), "{err}");
    }

    #[test]
    fn superseding_a_later_entry_is_rejected() {
        let raw = "[[thesis]]\ndate=\"2026-01-01\"\nclaim=\"A\"\nsupersedes=2\nbecause=\"x\"\n\n\
                   [[thesis]]\ndate=\"2026-02-01\"\nclaim=\"B\"\n";
        let err = parse(raw).unwrap_err().to_string();
        assert!(err.contains("not an earlier entry"), "{err}");
    }

    #[test]
    fn superseding_nothing_that_exists_is_rejected() {
        let raw = "[[thesis]]\ndate=\"2026-01-01\"\nclaim=\"A\"\nsupersedes=7\nbecause=\"x\"\n";
        let err = parse(raw).unwrap_err().to_string();
        assert!(err.contains("only 1 entries"), "{err}");
    }

    #[test]
    fn an_empty_file_has_no_current_thesis() {
        let f = parse("").unwrap();
        assert!(current(&f).is_err());
    }

    #[test]
    fn falsifiability_is_the_first_gap_asked_about() {
        let f = parse("[[thesis]]\ndate=\"2026-09-21\"\nclaim=\"Ship it.\"\n").unwrap();
        let g = gaps(current(&f).unwrap());
        assert_eq!(g[0].field, "test.falsified_by");
        assert_eq!(g.len(), 11, "every optional part should be asked after");
    }

    #[test]
    fn a_filled_field_is_not_asked_about() {
        let f = parse(FON).unwrap();
        let fields: Vec<&str> = gaps(current(&f).unwrap()).iter().map(|g| g.field).collect();
        assert!(!fields.contains(&"test.falsified_by"));
        assert!(!fields.contains(&"arc.where"));
        assert!(fields.contains(&"arc.why_now"));
        assert!(fields.contains(&"evidence.proven"));
    }

    #[test]
    fn the_pitch_leads_with_the_claim_and_omits_undeclared_sections() {
        let f = parse(FON).unwrap();
        let md = render_pitch_md(current(&f).unwrap(), "fon", "fid-thesis");
        assert!(md.starts_with("# fon\n\n> **No unverified alert ever reaches a responder.**"));
        assert!(md.contains("## Who it is for"));
        assert!(!md.contains("## Why now"), "undeclared sections stay out");
        assert!(md.contains("- ✗ No field deployment."));
    }

    #[test]
    fn the_generated_module_quotes_safely() {
        let raw = "[[thesis]]\ndate=\"2026-09-21\"\n\
                   claim=\"They said \\\"never\\\" — we\\nshipped.\"\n";
        let f = parse(raw).unwrap();
        let ts = render_ts(current(&f).unwrap(), "fid-thesis");
        assert!(
            ts.contains(r#"claim: "They said \"never\" — we shipped.""#),
            "{ts}"
        );
        assert!(ts.ends_with("} as const;\n"));
    }

    #[test]
    fn rendering_is_deterministic() {
        let f = parse(FON).unwrap();
        let t = current(&f).unwrap();
        assert_eq!(
            render_pitch_md(t, "fon", "fid-thesis"),
            render_pitch_md(t, "fon", "fid-thesis")
        );
        assert_eq!(render_ts(t, "fid-thesis"), render_ts(t, "fid-thesis"));
    }
}
