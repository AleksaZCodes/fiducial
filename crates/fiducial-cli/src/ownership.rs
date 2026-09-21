//! Who owns a scaffolded file — the product, or the platform.
//!
//! # The question this answers
//!
//! `fid new` and `fid add` write files into a product, and `fiducial.lock`
//! records a hash of each. `fid doctor` then reports every file whose hash no
//! longer matches as `modified since scaffold … run `fid upgrade` to
//! re-baseline`.
//!
//! For most of those files that report is wrong, and the templates say so
//! themselves. `MISSION.md`'s own footer reads:
//!
//! > It is **product-owned** — edit it freely.
//!
//! So the platform scaffolds a file, tells you to edit it, and then reports
//! your edit as drift. fon carries seventeen of these — `MISSION.md`,
//! `README.md`, `messages/*.json`, `content.toml`, `design-system.md`,
//! `apps/web/src/app/page.tsx` — which is why its `fid doctor` runs
//! `continue-on-error`. A check that fires on the normal state of a real
//! product is a check people switch off, and then it is not checking anything.
//!
//! # What re-baselining means, now that this exists
//!
//! It splits cleanly in two, and that is the whole point of drawing the line:
//!
//! - **Product-owned.** Editing it is the intended use, so there is nothing to
//!   re-baseline and nothing to report. The lock's `base_content` stays at the
//!   original scaffold *deliberately* — it is the merge base, and `fid upgrade`
//!   needs it to fold an upstream change into your edits. Re-baselining a
//!   product-owned file would throw away the only thing that makes that merge
//!   possible.
//!
//! - **Platform-owned.** The platform maintains it and `fid upgrade` will
//!   rewrite it. An edit here is real news, because it is about to be merged
//!   over or lost. This is the only case where re-baselining — recording your
//!   version as the new base, accepting the fork — is a coherent thing to want.
//!
//! So the old message was asking every product to re-baseline the one category
//! that must never be re-baselined, and staying quiet about nothing.
//!
//! # The default is platform-owned, and that direction is deliberate
//!
//! An unclassified path is treated as platform-owned, so a template nobody
//! thought about keeps warning. The other default fails silently: a
//! platform-owned file misfiled as product-owned stops reporting an edit that
//! `fid upgrade` is about to overwrite, and the first anyone hears of it is
//! their work disappearing.
//!
//! `every_scaffolded_path_is_classified_deliberately` holds the line: adding a
//! template to `SCAFFOLD_FILES` fails the test until someone decides which side
//! it is on.

/// Who is entitled to change a file after it is scaffolded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    /// Scaffolded as a starting point; editing it is the intended use.
    Product,
    /// The platform maintains it; `fid upgrade` rewrites it.
    Platform,
}

impl Ownership {
    pub fn is_product(self) -> bool {
        matches!(self, Ownership::Product)
    }
}

/// Paths a product owns, as exact names and directory prefixes.
///
/// Read this list as a set of claims about *who the file is for*, not about how
/// often it changes. `content.toml` might never be touched and is still the
/// product's; `scripts/derive-press.mjs` might be edited constantly and is
/// still the platform's, because editing it forks platform code.
const PRODUCT_OWNED: &[Rule] = &[
    // ── The product's own statements about itself ──
    //
    // Every one of these is scaffolded as a prompt to be replaced. `thesis.toml`
    // is scaffolded as a page of questions with no claim in it at all, so the
    // first thing anyone does to it is modify it.
    Rule::Exact("MISSION.md"),
    Rule::Exact("README.md"),
    Rule::Exact("ROADMAP.md"),
    Rule::Exact("thesis.toml"),
    Rule::Exact("design-system.md"),
    Rule::Exact("content.toml"),
    // Per-product agent context, and the template exists so a product can
    // describe its own layout and rules. fon's is entirely product-specific.
    // Calling it platform-owned would amount to telling every product not to
    // write its own agent instructions, which is the file's whole purpose.
    Rule::Exact("AGENTS.md"),
    // The scaffolded ignores are a starting set — Rust, Node, Fiducial
    // artifacts. Every real product adds to them: fon ignores `media/`, which
    // is exactly what a product is supposed to do here.
    Rule::Exact(".gitignore"),
    // The declaration file. `fid add` patches it, the product edits it, and
    // `[brand]`, `[adapters]` and `[i18n]` are *meant* to be filled in — it is
    // the single most-edited file in any product.
    Rule::Exact("fiducial.toml"),
    //
    // ── Facts the product authors, which pipelines read ──
    //
    // The catalogs are the copy declaration: a scaffolded `messages/en.json` is
    // a stub whose entire purpose is to be rewritten in the product's voice.
    Rule::Prefix("messages/"),
    Rule::Prefix("content/"),
    Rule::Prefix("press/"),
    Rule::Prefix("brand/"),
    Rule::Prefix("migrations/"),
    //
    // ── Pipeline declarations ──
    //
    // A pipeline's `outputs` say where this product's artifacts land, and a
    // product that moves its app moves those paths. The scaffolded list is a
    // default, not a rule — the `fid-thesis` pipeline's own comment says a
    // SvelteKit product should change the path.
    Rule::Prefix("pipelines/"),
    //
    // ── The application ──
    //
    // The whole point of `fid add app` is that you then build the app. Its
    // manifest gains dependencies, its pages gain content, its stylesheet gains
    // the product's own rules. Treating any of it as platform-owned would mean
    // reporting "you built your product" as drift.
    //
    // The exception below this list is `apps/web/src/generated/`, which is
    // derived rather than scaffolded and is guarded by `fid derive --check`.
    Rule::Prefix("apps/"),
];

/// Paths that stay platform-owned even though a broader rule above claims them.
///
/// Order matters: these are tested first.
const PLATFORM_OWNED_EXCEPTIONS: &[Rule] = &[
    // Derived, not scaffolded. Hand-editing one of these is already an error
    // and `fid derive --check` is what says so — with a better message than
    // this check could, because it knows which pipeline produces the file.
    Rule::Prefix("apps/web/src/generated/"),
    // Installed by `fid add` and overwritten by every `fid upgrade`. The
    // platform's own CLAUDE.md states this, and a product that edits one gets
    // its work silently replaced on the next upgrade — exactly the case the
    // warning needs to survive for.
    Rule::Prefix("apps/web/src/components/ui/"),
];

enum Rule {
    Exact(&'static str),
    Prefix(&'static str),
}

impl Rule {
    fn matches(&self, path: &str) -> bool {
        match self {
            Rule::Exact(p) => path == *p,
            Rule::Prefix(p) => path.starts_with(p),
        }
    }
}

/// Who owns the file at this product-relative path.
///
/// Paths use forward slashes, as everything in `fiducial.lock` does.
pub fn of(rel_path: &str) -> Ownership {
    if PLATFORM_OWNED_EXCEPTIONS
        .iter()
        .any(|r| r.matches(rel_path))
    {
        return Ownership::Platform;
    }
    if PRODUCT_OWNED.iter().any(|r| r.matches(rel_path)) {
        return Ownership::Product;
    }
    Ownership::Platform
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_a_product_is_told_to_edit_are_its_own() {
        // Each of these is scaffolded with a prompt in it, and each appeared in
        // fon's `fid doctor` as drift.
        for p in [
            "MISSION.md",
            "README.md",
            "thesis.toml",
            "fiducial.toml",
            "design-system.md",
            "content.toml",
            "messages/en.json",
            "messages/sr.json",
            "pipelines/brand.toml",
            "apps/web/src/app/page.tsx",
            "apps/web/src/app/globals.css",
            "apps/web/package.json",
        ] {
            assert_eq!(of(p), Ownership::Product, "{p}");
        }
    }

    #[test]
    fn what_the_platform_maintains_stays_the_platforms() {
        for p in [
            ".github/workflows/ci.yml",
            ".claude/settings.json",
            ".claude/agents/fiducial-review.md",
            ".claude/skills/design.md",
            ".fiducial/skills/i18n.md",
            // Derivation scripts a pipeline invokes. Editing one forks
            // platform code, and `fid upgrade` replaces it.
            "scripts/build-design-system.mjs",
            "scripts/derive-press.mjs",
        ] {
            assert_eq!(of(p), Ownership::Platform, "{p}");
        }
    }

    #[test]
    fn an_exception_beats_the_broader_rule_that_contains_it() {
        // `apps/` is product-owned, but not these two.
        assert_eq!(of("apps/web/src/app/layout.tsx"), Ownership::Product);
        assert_eq!(
            of("apps/web/src/generated/brand.ts"),
            Ownership::Platform,
            "derived files are guarded by `fid derive --check`, not by this"
        );
        assert_eq!(
            of("apps/web/src/components/ui/button.tsx"),
            Ownership::Platform,
            "shadcn copies are replaced on upgrade, so an edit is real news"
        );
        // A product's own component is still the product's.
        assert_eq!(
            of("apps/web/src/components/press-kit.tsx"),
            Ownership::Product
        );
    }

    #[test]
    fn an_unknown_path_defaults_to_platform_owned() {
        // The safe direction: a template nobody classified keeps warning. The
        // other default loses someone's work quietly on the next upgrade.
        assert_eq!(of("some/new/thing.txt"), Ownership::Platform);
    }

    #[test]
    fn a_prefix_rule_does_not_match_a_similarly_named_sibling() {
        assert_eq!(of("messages-backup/en.json"), Ownership::Platform);
        assert_eq!(of("pipelines.md"), Ownership::Platform);
    }

    #[test]
    fn every_scaffolded_path_is_classified_deliberately() {
        // The guard on this whole module: adding a template forces someone to
        // decide which side of the line it sits on, rather than inheriting the
        // default by accident.
        let classified: Vec<&str> = PRODUCT_OWNED
            .iter()
            .chain(PLATFORM_OWNED_EXCEPTIONS.iter())
            .filter_map(|r| match r {
                Rule::Exact(p) => Some(*p),
                Rule::Prefix(p) => Some(*p),
            })
            .collect();

        for (path, _) in crate::templates::SCAFFOLD_FILES
            .iter()
            .chain(crate::templates::FULL_ONLY_FILES.iter())
        {
            let named = classified
                .iter()
                .any(|c| *path == *c || path.starts_with(c) || c.starts_with(path));
            let platform_by_design = matches!(
                *path,
                ".claude/settings.json"
                    | ".claude/agents/fiducial-review.md"
                    | ".claude/agents/fiducial-design.md"
                    | ".github/workflows/ci.yml"
            );
            assert!(
                named || platform_by_design,
                "`{path}` is scaffolded but nothing here says who owns it. \
                 Add it to PRODUCT_OWNED, or to the platform list in this test."
            );
        }
    }
}
