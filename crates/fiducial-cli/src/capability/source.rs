//! Resolving a capability from outside the binary.
//!
//! Roadmap item 3, and the reason it is the keystone: *"We can add that later,
//! easily" is only true once shipping a capability does not require releasing
//! the CLI.*
//!
//! # What a source is
//!
//! ```sh
//! fid add stripe --from ./capabilities/stripe          # a directory
//! fid add stripe --from git:https://github.com/x/y     # a repository
//! fid add stripe --from git:https://…#v1.2.0           # pinned at a tag
//! ```
//!
//! Both land in the same place: a directory of files, handed to
//! `manifest::derive`. A third-party capability is not a second kind of thing
//! with its own install path — it is the same thing, resolved differently.
//!
//! # Why git and not npm or crates
//!
//! Design spec §3.3 names the source model directly: capabilities *"resolve
//! from the platform monorepo (first-party), or any git repo (third-party or
//! private) — the same source model the Claude Code marketplace already
//! supports."* A registry adds a packaging format — what a published
//! capability tarball contains, how its version resolves, which registry is
//! authoritative — and nothing needs that yet. Git covers private and public,
//! pins to a commit, and is already installed wherever `fid` is.
//!
//! # What is pinned
//!
//! A git source resolves to a **commit**, never a branch, and the commit is
//! what `fiducial.lock` records. `--from git:…#main` installs what `main` was
//! at that moment and says so; the next `fid add` of the same capability
//! without a revision does not silently become a different capability.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use super::manifest::{self, Capability, Source};

/// A source as a product writes it on the command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Spec {
    /// A directory on this machine.
    Path(PathBuf),
    /// A git repository, optionally at a revision.
    Git {
        url: String,
        /// Tag, branch or commit. `None` means the repository's default branch.
        rev: Option<String>,
        /// Subdirectory within the repository, when it holds several.
        subdir: Option<String>,
    },
}

impl Spec {
    /// Parse `--from`.
    ///
    /// `git:<url>[#<rev>][:<subdir>]` — anything else is a path. Prefixing
    /// rather than sniffing, because a URL and a path are not reliably
    /// distinguishable and a wrong guess reaches the network.
    pub fn parse(raw: &str) -> Result<Self> {
        let Some(rest) = raw.strip_prefix("git:") else {
            return Ok(Spec::Path(PathBuf::from(raw)));
        };

        // Split the subdirectory off the end first: a `#` can appear in
        // neither a git ref nor a path, but a `:` appears in every https URL.
        let (locator, subdir) = match rest.rsplit_once("::") {
            Some((l, s)) => (l, Some(s.to_string())),
            None => (rest, None),
        };
        let (url, rev) = match locator.rsplit_once('#') {
            Some((u, r)) => (u.to_string(), Some(r.to_string())),
            None => (locator.to_string(), None),
        };
        if url.is_empty() {
            bail!("`{raw}` names no repository");
        }
        Ok(Spec::Git { url, rev, subdir })
    }
}

/// Resolve a spec into a capability, fetching if necessary.
///
/// `cache` is where git checkouts land — inside the product, so a clone is
/// visible, removable, and not shared state some other product depends on.
pub fn resolve(spec: &Spec, id: &str, cache: &Path) -> Result<Capability> {
    match spec {
        Spec::Path(dir) => {
            if !dir.is_dir() {
                bail!("{} is not a directory", dir.display());
            }
            let cap = manifest::from_dir(dir, Source::Path(dir.display().to_string()))?;
            check_id(&cap, id)?;
            Ok(cap)
        }
        Spec::Git { url, rev, subdir } => {
            let checkout = clone(url, rev.as_deref(), cache)?;
            let resolved = head_commit(&checkout)?;
            let dir = match subdir {
                Some(s) => checkout.join(s),
                None => checkout.clone(),
            };
            if !dir.is_dir() {
                bail!(
                    "`{}` has no directory `{}`",
                    url,
                    subdir.as_deref().unwrap_or(".")
                );
            }
            let cap = manifest::from_dir(
                &dir,
                Source::Git {
                    url: url.clone(),
                    rev: resolved,
                },
            )?;
            check_id(&cap, id)?;
            Ok(cap)
        }
    }
}

/// The directory's name is the capability's id, and it has to be the one asked
/// for — otherwise `fid add stripe --from ./billing` installs `billing` under
/// the name `stripe`, and `fiducial.toml` records a capability that is not
/// what it says.
fn check_id(cap: &Capability, requested: &str) -> Result<()> {
    if cap.id != requested {
        bail!(
            "asked for `{requested}` but that source provides `{}`.\n\
             A capability's id is its directory name; rename the directory or \
             install it under its own name.",
            cap.id
        );
    }
    Ok(())
}

fn clone(url: &str, rev: Option<&str>, cache: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(cache).with_context(|| format!("creating {}", cache.display()))?;
    let dest = cache.join(slug(url));
    if dest.exists() {
        std::fs::remove_dir_all(&dest).with_context(|| format!("clearing {}", dest.display()))?;
    }

    // `--depth 1` for a named revision is not always valid (a bare commit sha
    // may be unreachable from any tip), so depth is only used when fetching a
    // default branch. Correctness over speed: a capability installed from the
    // wrong commit is worse than a slow clone.
    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--quiet");
    match rev {
        Some(r) => {
            cmd.arg("--branch").arg(r).arg("--depth").arg("1");
        }
        None => {
            cmd.arg("--depth").arg("1");
        }
    }
    let out = cmd
        .arg(url)
        .arg(&dest)
        .output()
        .context("running git clone — is git installed?")?;

    if !out.status.success() {
        // A commit sha is not a branch, so the `--branch` clone above fails for
        // one. Fall back to a full clone and check it out.
        if let Some(r) = rev {
            let full = Command::new("git")
                .args(["clone", "--quiet", url])
                .arg(&dest)
                .output()
                .context("running git clone")?;
            if full.status.success() {
                let co = Command::new("git")
                    .args(["-C"])
                    .arg(&dest)
                    .args(["checkout", "--quiet", r])
                    .output()
                    .context("running git checkout")?;
                if co.status.success() {
                    return Ok(dest);
                }
                bail!(
                    "`{r}` is not a revision in {url}:\n{}",
                    String::from_utf8_lossy(&co.stderr).trim()
                );
            }
        }
        bail!(
            "could not clone {url}:\n{}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(dest)
}

/// The commit actually checked out — what the lock pins, never a branch name.
fn head_commit(dir: &Path) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .context("running git rev-parse")?;
    if !out.status.success() {
        bail!("could not read HEAD of {}", dir.display());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// A filesystem-safe directory name for a clone URL.
fn slug(url: &str) -> String {
    url.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_argument_is_a_path() {
        assert_eq!(
            Spec::parse("./caps/stripe").unwrap(),
            Spec::Path(PathBuf::from("./caps/stripe"))
        );
        // Including one that looks URL-ish. Sniffing would reach the network
        // for something the user meant as a directory.
        assert!(matches!(
            Spec::parse("/srv/github.com/x/y").unwrap(),
            Spec::Path(_)
        ));
    }

    #[test]
    fn a_git_source_parses_url_revision_and_subdirectory() {
        let s = Spec::parse("git:https://github.com/x/y").unwrap();
        assert_eq!(
            s,
            Spec::Git {
                url: "https://github.com/x/y".into(),
                rev: None,
                subdir: None
            }
        );

        let s = Spec::parse("git:https://github.com/x/y#v1.2.0").unwrap();
        assert_eq!(
            s,
            Spec::Git {
                url: "https://github.com/x/y".into(),
                rev: Some("v1.2.0".into()),
                subdir: None
            }
        );

        let s = Spec::parse("git:https://github.com/x/y#main::caps/stripe").unwrap();
        assert_eq!(
            s,
            Spec::Git {
                url: "https://github.com/x/y".into(),
                rev: Some("main".into()),
                subdir: Some("caps/stripe".into())
            }
        );
    }

    /// The `https:` in a URL must not be read as a subdirectory separator.
    #[test]
    fn the_scheme_colon_is_not_a_subdirectory() {
        let s = Spec::parse("git:https://github.com/x/y").unwrap();
        match s {
            Spec::Git { url, subdir, .. } => {
                assert_eq!(url, "https://github.com/x/y");
                assert_eq!(subdir, None);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_empty_repository_is_refused() {
        assert!(Spec::parse("git:").is_err());
    }
}
