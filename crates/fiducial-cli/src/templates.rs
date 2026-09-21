//! Central registry of all embedded template content.
//!
//! Every template baked into the `fid` binary is accessible here by its
//! repo-relative path (forward slashes). `fid upgrade` uses this to detect
//! upstream template changes and produce 3-way merges.
//!
//! Templates are returned **unexpanded** — placeholders (`{{name}}`,
//! `{{version}}`) are not substituted. The caller must expand before comparing
//! against on-disk content (which was expanded at scaffold time).

// ── `fid new` templates ───────────────────────────────────────────────────────

const TMPL_FIDUCIAL_TOML: &str = include_str!("../templates/fiducial.toml.tmpl");
const TMPL_MISSION_MD: &str = include_str!("../templates/MISSION.md.tmpl");
const TMPL_THESIS_TOML: &str = include_str!("../templates/thesis.toml.tmpl");
const TMPL_PIPELINE_THESIS: &str = include_str!("../templates/pipelines-thesis.toml.tmpl");
const TMPL_AGENTS_MD: &str = include_str!("../templates/AGENTS.md.tmpl");
const TMPL_GITIGNORE: &str = include_str!("../templates/gitignore.tmpl");
const TMPL_CLAUDE_SETTINGS: &str = include_str!("../templates/claude-settings.json.tmpl");
const TMPL_AGENT_REVIEW: &str = include_str!("../templates/agents-fiducial-review.md.tmpl");
const TMPL_AGENT_DESIGN: &str = include_str!("../templates/agents-fiducial-design.md.tmpl");
const TMPL_CI: &str = include_str!("../templates/ci.yml.tmpl");
const TMPL_README: &str = include_str!("../templates/README.md.tmpl");
const TMPL_ROADMAP: &str = include_str!("../templates/ROADMAP.md.tmpl");

// ── Capability templates ──────────────────────────────────────────────────────

/// The platform's principles, extracted from `MISSION.md` by `build.rs`.
///
/// One declaration, one derivation. See `build.rs` for why this is generated
/// rather than a second copy kept honest by a test.
const PRINCIPLES: &str = include_str!(concat!(env!("OUT_DIR"), "/principles.md"));

// ── The scaffold set ─────────────────────────────────────────────────────────

/// Every file `fid new` writes, declared once.
///
/// This list is the single declaration of "what a scaffolded product contains".
/// `commands::new` iterates it to create a product; `commands::upgrade` diffs it
/// against `fiducial.lock` to find templates the platform has added since a
/// product was scaffolded, and installs them.
///
/// That second consumer is the reason this is a list rather than a sequence of
/// calls. Before it existed, a template added to `fid new` reached only products
/// scaffolded *afterwards* — `.github/workflows/ci.yml` would have been added to
/// the platform and never arrived in a single existing product. Principle 7: a
/// fix that cannot propagate is only half-finished.
pub const SCAFFOLD_FILES: &[(&str, &str)] = &[
    ("fiducial.toml", TMPL_FIDUCIAL_TOML),
    ("MISSION.md", TMPL_MISSION_MD),
    ("thesis.toml", TMPL_THESIS_TOML),
    ("pipelines/thesis.toml", TMPL_PIPELINE_THESIS),
    ("AGENTS.md", TMPL_AGENTS_MD),
    ("README.md", TMPL_README),
    (".gitignore", TMPL_GITIGNORE),
    (".claude/settings.json", TMPL_CLAUDE_SETTINGS),
    (".claude/agents/fiducial-review.md", TMPL_AGENT_REVIEW),
    (".claude/agents/fiducial-design.md", TMPL_AGENT_DESIGN),
    (".github/workflows/ci.yml", TMPL_CI),
];

/// Has the upstream template actually changed since it was installed?
///
/// # The bug this exists to fix
///
/// Both `fid doctor` and `fid upgrade` used to answer this by comparing the
/// stored base against `expand(raw, name, PLATFORM_VERSION)` — the template as
/// the *current* binary would write it. Most templates carry a `{{version}}`
/// stamp, so the moment the platform version moved, every one of them differed
/// by that stamp and nothing else.
///
/// The result was that a product appeared to have drifted from upstream on
/// files nobody upstream had touched. fon carried 33 such items, its
/// `fid doctor` is `continue-on-error` because of them, and `fid upgrade` would
/// open a 3-way merge on each one — which is how a product accumulates
/// conflicts in files that never changed.
///
/// So the comparison expands the template at the version the product recorded.
/// If the only difference was the stamp, the two match and there is nothing to
/// report; if the template text genuinely moved, they differ and it is real.
///
/// The stamp is itself a fact declared twice — `fiducial.lock` already records
/// `source_version` authoritatively — but removing it from every template is a
/// wider change than fixing the comparison, and the comparison was wrong on its
/// own terms.
pub fn upstream_changed(
    raw: &str,
    product_name: &str,
    installed_version: &str,
    base: &str,
) -> bool {
    expand(raw, product_name, installed_version) != base
}

/// Templates written only by `fid new --full`.
///
/// A roadmap for a product that has not decided what it claims is a form to
/// fill in, and being handed one before you have had the idea is the weight
/// this scaffold was carrying. `fid dash` already reports an absent roadmap as
/// an absence rather than an error, and there is a test for it — the state was
/// always supported, it just was not reachable.
///
/// Deliberately **not** in `SCAFFOLD_FILES`, and the difference matters.
/// `fid upgrade` updates whatever a product's lock already tracks, and uses
/// `SCAFFOLD_FILES` only to find templates the platform added since. So a
/// product created with `--full` keeps getting roadmap updates, and one created
/// minimal is never handed a roadmap it declined — which is the whole point of
/// declining it.
///
/// `.github/workflows/ci.yml` stays in the core set despite being on the same
/// list of things a small tool should not need. CI is what runs
/// `fid derive --check`, and a product that drifts from its own declarations is
/// worse than a product with one extra file. There is also no `fid add ci` to
/// recover it with, and a default you cannot undo is not a default.
pub const FULL_ONLY_FILES: &[(&str, &str)] = &[("ROADMAP.md", TMPL_ROADMAP)];

// ── Renamed templates ────────────────────────────────────────────────────────

/// Templates that have moved, as `(old path, new path)`.
///
/// A subagent's file name is its identity: `.claude/agents/design.md` registers
/// an agent called `design`, which collides with any other `design` agent the
/// user already has — from another plugin, another project, or their own global
/// config. Whichever loses the collision is simply unavailable, silently.
///
/// So the scaffolded agents are namespaced. That rename has to reach products
/// that already exist, and it cannot be a codemod: `MigrationOp` is a literal
/// search-and-replace within one file, and this is a file moving.
///
/// `fid upgrade` reads this list, installs the new path, and removes the old one
/// **only when it is unmodified** — a product that edited its copy keeps it, with
/// a warning, because silently deleting someone's edits to fix a naming problem
/// is a worse outcome than the naming problem.
pub const RENAMED_TEMPLATES: &[(&str, &str)] = &[
    (
        ".claude/agents/design.md",
        ".claude/agents/fiducial-design.md",
    ),
    (
        ".claude/agents/review.md",
        ".claude/agents/fiducial-review.md",
    ),
];

// ── Lookup ────────────────────────────────────────────────────────────────────

/// Look up the raw (unexpanded) template for a repo-relative path.
///
/// Scaffold templates only. For a file a capability installed, use
/// [`raw_for`] — which capability's version of a path is correct depends on
/// which capabilities the product has.
///
/// Returns `None` for paths that are not tracked as templates.
pub fn raw(rel_path: &str) -> Option<&'static str> {
    match rel_path {
        // fid new templates
        "fiducial.toml" => Some(TMPL_FIDUCIAL_TOML),
        "MISSION.md" => Some(TMPL_MISSION_MD),
        "thesis.toml" => Some(TMPL_THESIS_TOML),
        "pipelines/thesis.toml" => Some(TMPL_PIPELINE_THESIS),
        "AGENTS.md" => Some(TMPL_AGENTS_MD),
        ".gitignore" => Some(TMPL_GITIGNORE),
        ".claude/settings.json" => Some(TMPL_CLAUDE_SETTINGS),
        ".claude/agents/fiducial-review.md" => Some(TMPL_AGENT_REVIEW),
        ".claude/agents/fiducial-design.md" => Some(TMPL_AGENT_DESIGN),
        ".github/workflows/ci.yml" => Some(TMPL_CI),
        "README.md" => Some(TMPL_README),
        "ROADMAP.md" => Some(TMPL_ROADMAP),
        _ => None,
    }
}

/// Look up a template, scaffold or capability, for a product.
///
/// The capability half used to be 86 `include_str!` constants and a 40-line
/// match, sitting next to the directories that already held exactly those
/// files. It is now read from the capability registry, which `build.rs` derives
/// from those same directories.
///
/// # Why this takes the product's capability list
///
/// `firmware/Cargo.toml`, `firmware/rust-toolchain.toml` and
/// `firmware/shared/src/lib.rs` are installed by **both** firmware
/// capabilities, with different content. The old match had one arm each,
/// always resolving to the RP2040 version — so a product with `firmware-stm32`
/// had `fid doctor` and `fid upgrade` comparing its files against the wrong
/// board's template. Silent, and wrong in the direction that matters: it
/// reports drift that is not drift, and would overwrite a correct file on
/// upgrade.
///
/// Resolving against the capabilities the product actually installed removes
/// the ambiguity in every real case. Where a path is still ambiguous — two
/// installed capabilities shipping the same path with different content — this
/// returns `None` rather than guessing, and the caller skips the file. A
/// comparison against the wrong template is worse than no comparison.
pub fn raw_for(rel_path: &str, enabled: &[String]) -> Option<&'static str> {
    if let Some(t) = raw(rel_path) {
        return Some(t);
    }

    let mut found: Option<&'static str> = None;
    for id in enabled {
        let Some(cap) = crate::capability::find(id) else {
            continue;
        };
        let Some(content) = capability_file(cap, rel_path) else {
            continue;
        };
        match found {
            Some(existing) if existing != content => return None,
            _ => found = Some(content),
        }
    }
    found
}

/// The content a capability installs at `rel_path`, across all three kinds.
fn capability_file(
    cap: &'static crate::capability::Capability,
    rel_path: &str,
) -> Option<&'static str> {
    use crate::capability::Declaration;

    cap.declarations
        .iter()
        .find_map(|d| match d {
            Declaration::File(f) if f.path == rel_path => Some(f.content.as_str()),
            _ => None,
        })
        .or_else(|| {
            cap.pipelines
                .iter()
                .chain(cap.templates.iter())
                .find(|f| f.path == rel_path)
                .map(|f| f.content.as_str())
        })
}

/// Expand `{{name}}` and `{{version}}` placeholders in a raw template.
pub fn expand(raw_template: &str, product_name: &str, platform_version: &str) -> String {
    raw_template
        .replace("{{name}}", product_name)
        .replace("{{version}}", platform_version)
        // The principles are authored once, in the platform's MISSION.md, and
        // extracted by build.rs. A scaffolded product cannot link to that file —
        // its own MISSION.md says what the product is for — so it receives the
        // text. Generated, never hand-copied.
        .replace("{{principles}}", PRINCIPLES)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape that caused the false drift: a template whose only
    /// version-dependent content is the stamp in its footer.
    const STAMPED: &str = "# {{name}}\n\nBody that never changes.\n\n\
                           _Scaffolded by `fid new` (fiducial {{version}})._\n";

    #[test]
    fn a_version_stamp_alone_is_not_an_upstream_change() {
        // fon installed at 0.1.0 and the platform is now far past it. Nothing
        // upstream moved, so nothing should be reported — this is the bug that
        // put 20 phantom items in `fid doctor` and made `fid upgrade` open a
        // 3-way merge on files nobody had touched.
        let base = expand(STAMPED, "fon", "0.1.0");
        assert!(!upstream_changed(STAMPED, "fon", "0.1.0", &base));
    }

    #[test]
    fn comparing_at_the_current_version_is_what_produced_the_phantom_drift() {
        // The old comparison, kept as a test so the regression is named: it
        // reports a change where there is none.
        let base = expand(STAMPED, "fon", "0.1.0");
        assert_ne!(
            expand(STAMPED, "fon", env!("CARGO_PKG_VERSION")),
            base,
            "the stamp alone differs — which is exactly why it must not be the comparison"
        );
    }

    #[test]
    fn a_real_edit_to_the_template_is_still_reported() {
        let base = expand(STAMPED, "fon", "0.1.0");
        let moved = STAMPED.replace("Body that never changes.", "Body that did change.");
        assert!(upstream_changed(&moved, "fon", "0.1.0", &base));
    }

    #[test]
    fn a_template_with_no_stamp_is_unaffected_either_way() {
        let plain = "# {{name}}\n\nNo stamp here.\n";
        let base = expand(plain, "fon", "0.1.0");
        assert!(!upstream_changed(plain, "fon", "0.9.9", &base));
        let moved = plain.replace("No stamp here.", "Changed.");
        assert!(upstream_changed(&moved, "fon", "0.9.9", &base));
    }

    #[test]
    fn the_product_name_still_participates_in_the_comparison() {
        // Ownership of the comparison moved, not its inputs: a base recorded
        // for another product must not read as unchanged.
        let base = expand(STAMPED, "other-product", "0.1.0");
        assert!(upstream_changed(STAMPED, "fon", "0.1.0", &base));
    }
}
