//! A capability is a **directory**, and its manifest is **derived from that
//! directory** rather than hand-written.
//!
//! Both halves are design-spec §3.3, and the second is the load-bearing one:
//!
//! > *The minimum viable capability is one file. This is a hard design
//! > constraint, not a nicety: the failure mode for an extension system is
//! > ceremony, and a capability that needs a manifest, a skill, tests, docs and
//! > migrations before it does anything is one nobody creates. So: only
//! > `SKILL.md` is required.*
//!
//! # The layout, and what each part becomes
//!
//! ```text
//! <capability>/
//! ├── SKILL.md          REQUIRED — and the only required file
//! ├── capability.toml   optional — the facts a layout cannot carry
//! ├── declarations/…    typed facts, installed at the path *below* declarations/
//! ├── pipelines/*.toml  derivations, gated by `fid derive --check`
//! └── anything else     templates, installed at their own path
//! ```
//!
//! `pipelines/` needs no convention invented for it: that is already where
//! `pipeline::discover` looks inside a product, so a capability's pipeline
//! directory and a product's are the same directory.
//!
//! `declarations/` is the one piece the layout could not otherwise express. A
//! declaration and a template are both files copied into a product; what
//! separates them is whether two different pipelines could read it and both be
//! correct. That is not visible in a path, so the path says it.
//!
//! # Why one derivation, over bytes
//!
//! This function takes `(path, content)` pairs rather than a directory, so the
//! **same** code derives a built-in capability from bytes embedded at build
//! time and a third-party one from a directory on disk. A second construction
//! path would be a second definition of what a capability is, and the two would
//! answer differently the first time either changed.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// One file a capability installs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Path relative to the product root, forward slashes.
    pub path: String,
    /// File content, with `{{name}}` and `{{version}}` still unexpanded.
    pub content: String,
}

/// A fact a capability introduces, which its pipelines then read.
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    /// A file in the product that people edit and pipelines read.
    File(FileEntry),
    /// A `fiducial.toml` block, seeded on install when the product has not
    /// already declared it.
    ///
    /// The seed is TOML data rather than a Rust function. It used to be
    /// `fn(&mut Config)`, which no capability outside this binary could ever
    /// supply — the signature quietly made "capabilities are resolvable from
    /// outside" false for every capability that declares a block.
    ConfigBlock {
        /// Block name as it appears in `fiducial.toml`, e.g. `i18n`.
        name: String,
        /// What to write into the block when it is absent or empty.
        ///
        /// `None` when the pipeline reads a block that is already present in
        /// every scaffold (e.g. `[adapters]`) and needs no initial seed. The
        /// install logic skips writing when this is `None`.
        seed: Option<toml::Value>,
    },
    /// A directory of facts the **product** authors, which a pipeline reads.
    ///
    /// `migrations/` is the case this exists for: the capability ships no
    /// migration and cannot, because which migrations a product needs is the
    /// product's business — but its pipeline reads every file in there, so
    /// "declares nothing for its pipeline to read" was both true and wrong.
    ///
    /// Shipping a placeholder file instead would have been worse: a dummy
    /// `0000_init.sql` is a migration that runs.
    Directory {
        /// Path in the product, e.g. `migrations`.
        path: String,
    },
}

impl Declaration {
    /// What this declaration is called in `fid capability list` and `fid dash`.
    pub fn name(&self) -> &str {
        match self {
            Self::File(f) => &f.path,
            Self::ConfigBlock { name, .. } => name,
            Self::Directory { path } => path,
        }
    }
}

/// Where a capability came from.
///
/// Recorded so `fid capability list` can say it and `fiducial.lock` can pin it.
/// A product that installed `stripe` from someone's repository should be able
/// to find out whose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Shipped inside this `fid` binary.
    Builtin,
    /// A directory on this machine.
    Path(String),
    /// A git repository at a resolved revision.
    Git {
        /// Clone URL as the product declared it.
        url: String,
        /// The commit actually installed — never a branch name.
        rev: String,
    },
}

impl Source {
    /// How the source reads in `fid capability list` and the lock.
    pub fn label(&self) -> String {
        match self {
            Self::Builtin => "built-in".to_string(),
            Self::Path(p) => format!("path:{p}"),
            Self::Git { url, rev } => {
                format!("git:{url}#{}", rev.chars().take(12).collect::<String>())
            }
        }
    }
}

/// A capability, however it was resolved.
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: String,
    pub description: String,
    pub declarations: Vec<Declaration>,
    pub pipelines: Vec<FileEntry>,
    pub requires_adapters: Vec<String>,
    /// External commands this capability's workflow needs on PATH.
    ///
    /// A capability can install every file it owns and still not work: `deploy`
    /// derives a `wrangler.toml` that only `wrangler` can act on, and a product
    /// installing `fid` in CI from a GitHub remote needs `gh`. That is as much
    /// a dependency as an adapter contract — and it is the kind that announces
    /// itself as a command-not-found halfway through a release rather than at
    /// the moment the capability is installed.
    pub requires_tools: Vec<String>,
    /// Other capabilities this one cannot work without.
    ///
    /// Not a convenience. A capability that derives user-visible copy is
    /// incomplete without `i18n`, because copy is a fact and a fact has one
    /// declaration and one derivation per locale (MISSION.md 1c). Installed
    /// alone, such a capability quietly produces a monolingual artifact and the
    /// product only discovers it when someone who reads the other language
    /// looks at the page — which is the exact failure 1c exists to prevent, so
    /// it is caught at install instead.
    pub requires_capabilities: Vec<String>,
    pub guard_rules: Vec<String>,
    pub templates: Vec<FileEntry>,
    pub skill_md: String,
    pub source: Source,
}

/// The optional `capability.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    guard_rules: Vec<String>,
    #[serde(default)]
    requires_adapters: Vec<String>,
    #[serde(default)]
    requires_tools: Vec<String>,
    #[serde(default)]
    requires_capabilities: Vec<String>,
    #[serde(default)]
    declarations: ManifestDeclarations,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestDeclarations {
    /// Declarations that are `fiducial.toml` blocks rather than files.
    #[serde(default)]
    config: Vec<ConfigDeclaration>,
    /// Directories the product fills and a pipeline reads.
    #[serde(default)]
    directory: Vec<DirectoryDeclaration>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DirectoryDeclaration {
    path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigDeclaration {
    block: String,
    #[serde(default)]
    seed: Option<toml::Value>,
}

/// The one required file.
pub const SKILL_FILE: &str = "SKILL.md";
/// The optional manifest.
pub const MANIFEST_FILE: &str = "capability.toml";
/// Files under here are declarations, installed at the path below it.
pub const DECLARATIONS_DIR: &str = "declarations/";
/// Files under here are pipelines.
pub const PIPELINES_DIR: &str = "pipelines/";

/// Files a capability directory carries that are *about* it rather than part of
/// what it installs.
const NOT_INSTALLED: &[&str] = &[SKILL_FILE, MANIFEST_FILE, "README.md"];

/// Derive a capability from the files of its directory.
///
/// `files` maps capability-relative path → content. `id` is the directory name.
pub fn derive(id: &str, files: &BTreeMap<String, String>, source: Source) -> Result<Capability> {
    let skill_md = files.get(SKILL_FILE).cloned().with_context(|| {
        format!(
            "capability `{id}` has no {SKILL_FILE}.\n\
                 It is the one required file: a capability shipping content without agent \
                 instructions is one an agent reverse-engineers every session (design spec §3.3)."
        )
    })?;

    let manifest: Manifest = match files.get(MANIFEST_FILE) {
        Some(raw) => toml::from_str(raw)
            .with_context(|| format!("parsing {MANIFEST_FILE} of capability `{id}`"))?,
        None => Manifest::default(),
    };

    let mut declarations: Vec<Declaration> = manifest
        .declarations
        .config
        .into_iter()
        .map(|c| Declaration::ConfigBlock {
            name: c.block,
            seed: c.seed,
        })
        .collect();
    declarations.extend(
        manifest
            .declarations
            .directory
            .into_iter()
            .map(|d| Declaration::Directory { path: d.path }),
    );
    let mut pipelines = Vec::new();
    let mut templates = Vec::new();

    for (path, content) in files {
        if NOT_INSTALLED.contains(&path.as_str()) {
            continue;
        }
        if let Some(rest) = path.strip_prefix(DECLARATIONS_DIR) {
            if rest.is_empty() {
                continue;
            }
            declarations.push(Declaration::File(FileEntry {
                path: rest.to_string(),
                content: content.clone(),
            }));
        } else if path.starts_with(PIPELINES_DIR) {
            pipelines.push(FileEntry {
                path: path.clone(),
                content: content.clone(),
            });
        } else {
            templates.push(FileEntry {
                path: path.clone(),
                content: content.clone(),
            });
        }
    }

    // A description is needed by `fid capability list`, and requiring
    // `capability.toml` for it alone would break the one-file minimum. So it
    // falls back to the skill's own first line of prose.
    let description = manifest
        .description
        .unwrap_or_else(|| description_from_skill(&skill_md, id));

    let cap = Capability {
        id: id.to_string(),
        description,
        declarations,
        pipelines,
        requires_adapters: manifest.requires_adapters,
        requires_tools: manifest.requires_tools,
        requires_capabilities: manifest.requires_capabilities,
        guard_rules: manifest.guard_rules,
        templates,
        skill_md,
        source,
    };
    validate_paths(&cap)?;
    Ok(cap)
}

/// The first line of real prose in a skill, for a capability with no manifest.
fn description_from_skill(skill: &str, id: &str) -> String {
    skill
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("---"))
        .map(|l| l.trim_matches('*').to_string())
        .unwrap_or_else(|| format!("the {id} capability"))
}

/// No installed path may escape the product root.
///
/// A capability is content someone else wrote, and `fid add` writes it into
/// your repository. `../../.ssh/authorized_keys` is a path a manifest could
/// ask for, and an absolute path is one a careless template could carry.
/// Refusing both here means every source — built-in, path, git — is checked by
/// the same rule, because they all arrive through this function.
fn validate_paths(cap: &Capability) -> Result<()> {
    let all = cap
        .declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::File(f) => Some(&f.path),
            // A directory is created in the product, so `../../.ssh` is the
            // same escape here as it is for a file.
            Declaration::Directory { path } => Some(path),
            Declaration::ConfigBlock { .. } => None,
        })
        .chain(cap.pipelines.iter().map(|f| &f.path))
        .chain(cap.templates.iter().map(|f| &f.path));

    for path in all {
        if path.starts_with('/') || path.starts_with('\\') || path.contains(':') {
            bail!("capability `{}` installs an absolute path `{path}`", cap.id);
        }
        if path.split('/').any(|seg| seg == "..") {
            bail!(
                "capability `{}` installs `{path}`, which escapes the product root",
                cap.id
            );
        }
        if path.split('/').any(|seg| seg == ".git") {
            bail!("capability `{}` writes into `.git` via `{path}`", cap.id);
        }
    }
    Ok(())
}

/// Read a capability directory from disk.
///
/// The filesystem half of `derive`: walk the directory into the same
/// `(path, content)` map the embedded built-ins provide.
pub fn from_dir(dir: &std::path::Path, source: Source) -> Result<Capability> {
    let id = dir
        .file_name()
        .and_then(|s| s.to_str())
        .with_context(|| format!("capability directory has no name: {}", dir.display()))?
        .to_string();

    let mut files = BTreeMap::new();
    collect(dir, dir, &mut files)
        .with_context(|| format!("reading capability directory {}", dir.display()))?;

    if files.is_empty() {
        bail!(
            "{} is empty — a capability needs at least a {SKILL_FILE}",
            dir.display()
        );
    }
    derive(&id, &files, source)
}

fn collect(
    root: &std::path::Path,
    dir: &std::path::Path,
    out: &mut BTreeMap<String, String>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') && name != ".cargo" {
            // `.git`, `.DS_Store` and friends are the checkout, not the
            // capability. `.cargo/config.toml` genuinely is content.
            continue;
        }
        if path.is_dir() {
            collect(root, &path, out)?;
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .expect("walked below root")
            .to_string_lossy()
            .replace('\\', "/");
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("{} is not UTF-8 text", path.display()))?;
        out.insert(rel, content);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(p, c)| ((*p).to_string(), (*c).to_string()))
            .collect()
    }

    const SKILL: &str =
        "# Skill: x\n\nWhat this capability is for, at length enough to be useful.\n";

    /// Design spec §3.3: *only `SKILL.md` is required.*
    ///
    /// The failure mode for an extension system is ceremony. A capability that
    /// needs a manifest before it does anything is one nobody creates.
    #[test]
    fn one_file_is_a_complete_capability() {
        let cap = derive("x", &files(&[("SKILL.md", SKILL)]), Source::Builtin).unwrap();
        assert_eq!(cap.id, "x");
        assert!(cap.declarations.is_empty());
        assert!(cap.pipelines.is_empty());
        assert!(cap.templates.is_empty());
        // Derived from the skill, so no manifest is needed for a description.
        assert!(cap.description.contains("What this capability is for"));
    }

    #[test]
    fn without_a_skill_it_is_not_a_capability() {
        let err = derive("x", &files(&[("README.md", "hi")]), Source::Builtin).unwrap_err();
        assert!(format!("{err:#}").contains("SKILL.md"));
    }

    /// The kind of each file comes from where it sits.
    #[test]
    fn the_layout_decides_what_each_file_is() {
        let cap = derive(
            "x",
            &files(&[
                ("SKILL.md", SKILL),
                ("declarations/data/facts.json", "{}"),
                ("pipelines/x.toml", "name = \"x\""),
                ("apps/thing/config.toml", "k = 1"),
                ("README.md", "docs"),
            ]),
            Source::Builtin,
        )
        .unwrap();

        // A declaration installs at the path *below* declarations/.
        assert_eq!(cap.declarations.len(), 1);
        assert_eq!(cap.declarations[0].name(), "data/facts.json");
        assert_eq!(cap.pipelines.len(), 1);
        assert_eq!(cap.pipelines[0].path, "pipelines/x.toml");
        assert_eq!(cap.templates.len(), 1);
        assert_eq!(cap.templates[0].path, "apps/thing/config.toml");
        // README is about the capability, not part of what it installs.
        assert!(!cap.templates.iter().any(|f| f.path == "README.md"));
    }

    /// A config block is a declaration with no file, expressed as data.
    ///
    /// It used to be `fn(&mut Config)`, which no capability outside the binary
    /// could supply — the signature quietly made "resolvable from outside"
    /// false for every capability that declares a block.
    #[test]
    fn a_config_block_declaration_is_data() {
        let manifest = r#"
description = "d"
[[declarations.config]]
block = "billing"
seed = { currency = "EUR" }
"#;
        let cap = derive(
            "x",
            &files(&[("SKILL.md", SKILL), ("capability.toml", manifest)]),
            Source::Builtin,
        )
        .unwrap();
        match &cap.declarations[0] {
            Declaration::ConfigBlock { name, seed } => {
                assert_eq!(name, "billing");
                let seed = seed.as_ref().expect("seed present");
                assert_eq!(seed["currency"].as_str(), Some("EUR"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_manifest_key_nobody_recognises_is_an_error_not_a_shrug() {
        // `deny_unknown_fields`: a misspelled key that is silently ignored is a
        // capability that does not do what its author wrote down.
        let err = derive(
            "x",
            &files(&[
                ("SKILL.md", SKILL),
                (
                    "capability.toml",
                    "description = \"d\"\nguard_rule = [\"x\"]\n",
                ),
            ]),
            Source::Builtin,
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("guard_rule"), "{err:#}");
    }

    // ── Trust ────────────────────────────────────────────────────────────────
    //
    // A capability is content someone else wrote, and `fid add` writes it into
    // your repository. Every source goes through `derive`, so checking here
    // covers built-in, path and git at once.

    #[test]
    fn a_path_that_escapes_the_product_root_is_refused() {
        let err = derive(
            "x",
            &files(&[("SKILL.md", SKILL), ("../../.ssh/authorized_keys", "key")]),
            Source::Path("somewhere".into()),
        )
        .unwrap_err();
        assert!(
            format!("{err:#}").contains("escapes the product root"),
            "{err:#}"
        );
    }

    #[test]
    fn an_absolute_path_is_refused() {
        let err = derive(
            "x",
            &files(&[("SKILL.md", SKILL), ("/etc/cron.d/job", "* * * * * root x")]),
            Source::Path("somewhere".into()),
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("absolute path"), "{err:#}");
    }

    #[test]
    fn writing_into_dot_git_is_refused() {
        let err = derive(
            "x",
            &files(&[("SKILL.md", SKILL), (".git/hooks/pre-commit", "#!/bin/sh")]),
            Source::Path("somewhere".into()),
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains(".git"), "{err:#}");
    }

    #[test]
    fn a_source_says_where_a_capability_came_from() {
        assert_eq!(Source::Builtin.label(), "built-in");
        assert_eq!(Source::Path("./x".into()).label(), "path:./x");
        assert_eq!(
            Source::Git {
                url: "https://example.com/x".into(),
                rev: "0123456789abcdef0123456789abcdef01234567".into(),
            }
            .label(),
            "git:https://example.com/x#0123456789ab"
        );
    }
}
