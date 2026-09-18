//! `fid repo protect` — make the remote enforce what `[guard]` already claims.
//!
//! # The gap this closes
//!
//! `no-direct-main-push` is a local hook. It fires on `git push` from a machine
//! that has the hook installed — and on no other machine, and on nothing a
//! token or a CI job does. A product declaring it believes main is protected
//! while GitHub happily accepts a push to it.
//!
//! That is the same failure this platform already refuses to shrug at one layer
//! in: a guard rule listed with no implementation is a finding, because "a
//! product listing it believes it is guarded and is not". The remote is the
//! other half of that sentence, and it went unsaid until someone looked.
//!
//! # What it applies, and where that comes from
//!
//! Nothing here is a preference. Every setting is derived from something the
//! product has already declared:
//!
//! | Setting | Derived from |
//! |---|---|
//! | required status checks | the job names in `.github/workflows/*.yml` |
//! | pull request required | `[guard] no-direct-main-push` |
//! | force pushes blocked | the same rule — a force push is a push |
//! | deletions blocked | the same rule — deleting main is the limit case |
//!
//! Required checks are read from the workflows rather than typed here, because
//! a list of check names in two places is a list that goes stale on the next
//! renamed job — and the failure mode is a branch that reports protected while
//! requiring a check nothing produces, which blocks every PR.

use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::config::Config;

/// What the remote should enforce, derived from the product.
struct Policy {
    slug: String,
    /// Job names CI produces, which become the required checks.
    checks: Vec<String>,
}

pub fn run(apply: bool, except: &[String]) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = Config::find_root(&cwd)?;
    let cfg = Config::load(&root.join(crate::config::CONFIG_FILE))
        .context("fid repo needs fiducial.toml")?;

    if !cfg.guard.rules.iter().any(|r| r == "no-direct-main-push") {
        bail!(
            "this product does not declare `no-direct-main-push` in [guard] rules.\n  \
             Protection here is derived from that declaration — add it first, so the \
             local hook and the remote say the same thing."
        );
    }

    let mut policy = derive_policy(&root)?;

    // A job that is a gate and a job that is advice look identical in a
    // workflow file — `continue-on-error` is the only signal, and a review job
    // that needs an API key does not carry it. Required, such a job makes every
    // PR unmergeable the day the secret is missing or rate-limited.
    //
    // So the derivation stays complete and the exclusion is explicit: named on
    // the command line, printed in the output, never guessed from the name.
    let mut excluded: Vec<String> = Vec::new();
    if !except.is_empty() {
        policy.checks.retain(|c| {
            let drop = except.iter().any(|e| e == c);
            if drop {
                excluded.push(c.clone());
            }
            !drop
        });
        for e in except {
            if !excluded.iter().any(|x| x == e) {
                bail!(
                    "--except {e:?} matches no job in .github/workflows/.\n  \
                     Run without --apply to see the names as they are derived."
                );
            }
        }
    }

    println!("✦ fid repo protect — {}", policy.slug);
    println!();
    println!("  derived from this product's declarations:");
    println!("    · pull request required before merge   [guard] no-direct-main-push");
    println!("    · force pushes blocked                 [guard] no-direct-main-push");
    println!("    · branch deletion blocked              [guard] no-direct-main-push");
    if policy.checks.is_empty() {
        println!("    · no required checks — no workflow job names found");
    } else {
        println!(
            "    · {} required check(s)                  .github/workflows/",
            policy.checks.len()
        );
        for c in &policy.checks {
            println!("        {c}");
        }
    }
    for e in &excluded {
        println!("    · not required: {e}                   --except");
    }
    println!();

    if !apply {
        println!("  Nothing changed. Re-run with --apply to write this to GitHub.");
        return Ok(());
    }

    apply_policy(&policy)?;
    println!("  ✓ main is protected on {}", policy.slug);
    Ok(())
}

fn derive_policy(root: &Path) -> Result<Policy> {
    let slug = github_slug(root).ok_or_else(|| {
        anyhow::anyhow!(
            "no GitHub `origin` remote — there is nothing to protect.\n  \
             `fid repo protect` configures GitHub; a product hosted elsewhere \
             needs its own host's equivalent."
        )
    })?;

    Ok(Policy {
        slug,
        checks: workflow_job_names(root),
    })
}

/// The `name:` of every job in `.github/workflows/*.yml`.
///
/// Parsed rather than declared, because a required check whose name nothing
/// produces blocks every PR forever, and a hand-kept list is exactly the thing
/// that survives a renamed job.
///
/// Deliberately shallow: this reads `name:` under a two-space indent inside
/// `jobs:`, which is the shape every workflow in a scaffolded product has. A
/// workflow it cannot read contributes nothing rather than contributing a
/// wrong name — `--apply` prints what it found before it writes.
fn workflow_job_names(root: &Path) -> Vec<String> {
    let dir = root.join(".github/workflows");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    let mut files: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    files.sort();

    for path in files {
        if !path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
            continue;
        }
        // Release workflows run on merge, not on a PR, so a required check by
        // that name would never report and would block every PR.
        if path.file_stem().is_some_and(|s| s == "release") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut in_jobs = false;
        // The name of the job currently being read, and whether it has
        // declared itself advisory. A job is only collected once its block
        // ends, because `continue-on-error` may appear after `name`.
        let mut current: Option<String> = None;
        let mut advisory = false;

        let flush = |current: &mut Option<String>, advisory: &mut bool, names: &mut Vec<String>| {
            if let Some(n) = current.take() {
                if !*advisory && !names.contains(&n) {
                    names.push(n);
                }
            }
            *advisory = false;
        };

        for line in text.lines() {
            if line.starts_with("jobs:") {
                in_jobs = true;
                continue;
            }
            if in_jobs && !line.starts_with(' ') && !line.trim().is_empty() {
                flush(&mut current, &mut advisory, &mut names);
                in_jobs = false;
            }
            if !in_jobs {
                continue;
            }
            let t = line.trim_start();
            let indent = line.len() - t.len();

            // `  build:` — two spaces: a new job begins, so the previous one is
            // complete and can be judged.
            if indent == 2 && t.ends_with(':') && !t.starts_with('#') {
                flush(&mut current, &mut advisory, &mut names);
                continue;
            }
            // `    name: web build` — four spaces, inside a job block.
            if indent == 4 {
                if let Some(v) = t.strip_prefix("name:") {
                    let v = v.trim().trim_matches(['"', '\'']);
                    if !v.is_empty() {
                        current = Some(v.to_string());
                    }
                }
                // A job that declares itself advisory is not a gate. This is
                // the workflow saying so in the one place that already means
                // it — rather than this command guessing from a job's name,
                // which is how "Opus review" and "review coverage" end up
                // being treated the same.
                if t.starts_with("continue-on-error:") && t.contains("true") {
                    advisory = true;
                }
            }
        }
        flush(&mut current, &mut advisory, &mut names);
    }
    names
}

fn apply_policy(policy: &Policy) -> Result<()> {
    // The REST shape wants an explicit null for the settings being left off,
    // and `gh api` sends a JSON body from stdin with `--input -`.
    let checks = serde_json::json!({
        "strict": true,
        "contexts": policy.checks,
    });
    let body = serde_json::json!({
        "required_status_checks": if policy.checks.is_empty() {
            serde_json::Value::Null
        } else {
            checks
        },
        // One approving review would lock a solo maintainer out of their own
        // repository — the rule being enforced is "go through a PR", not "find
        // a second person".
        "required_pull_request_reviews": { "required_approving_review_count": 0 },
        "enforce_admins": false,
        "restrictions": serde_json::Value::Null,
        "allow_force_pushes": false,
        "allow_deletions": false,
    });

    let mut child = std::process::Command::new("gh")
        .args([
            "api",
            "--method",
            "PUT",
            &format!("repos/{}/branches/main/protection", policy.slug),
            "--input",
            "-",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("spawning `gh` — is it installed and logged in?")?;

    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin was piped");
        stdin.write_all(body.to_string().as_bytes())?;
    }

    let out = child.wait_with_output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!(
            "gh refused to set protection:\n  {}\n  \
             Protection needs admin rights on the repository.",
            err.trim()
        );
    }
    Ok(())
}

fn github_slug(root: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let rest = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))?;
    Some(rest.trim_end_matches(".git").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workflows(files: &[(&str, &str)]) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(".github/workflows");
        std::fs::create_dir_all(&dir).unwrap();
        for (name, body) in files {
            std::fs::write(dir.join(name), body).unwrap();
        }
        tmp
    }

    const CI: &str = "name: CI\n\non:\n  pull_request:\n\njobs:\n  fiducial:\n    name: fiducial checks\n    runs-on: ubuntu-latest\n  web:\n    name: web build\n    runs-on: ubuntu-latest\n";

    #[test]
    fn job_names_come_from_the_workflows() {
        let tmp = workflows(&[("ci.yml", CI)]);
        let names = workflow_job_names(tmp.path());
        assert_eq!(names, vec!["fiducial checks", "web build"]);
    }

    #[test]
    fn the_workflow_name_is_not_a_job_name() {
        // `name: CI` at the top is the workflow, not a check. Requiring it
        // would block every PR on a status nothing reports.
        let tmp = workflows(&[("ci.yml", CI)]);
        assert!(!workflow_job_names(tmp.path()).contains(&"CI".to_string()));
    }

    #[test]
    fn release_workflows_are_skipped() {
        // Release runs on merge to main, never on a PR, so requiring its job
        // would make main unmergeable.
        let tmp = workflows(&[
            ("ci.yml", CI),
            (
                "release.yml",
                "name: Release\n\njobs:\n  release:\n    name: Release\n",
            ),
        ]);
        let names = workflow_job_names(tmp.path());
        assert!(!names.contains(&"Release".to_string()), "{names:?}");
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn a_job_that_declares_itself_advisory_is_not_required() {
        // A review is an opinion. Required, it blocks a green branch the day
        // the API key is missing — and `continue-on-error` is the workflow
        // already saying so, in the one place that means it.
        let tmp = workflows(&[(
            "review.yml",
            "name: Claude Review\n\njobs:\n  review:\n    name: Opus review\n    runs-on: ubuntu-latest\n    continue-on-error: true\n",
        )]);
        assert!(workflow_job_names(tmp.path()).is_empty());
    }

    #[test]
    fn advisory_on_one_job_does_not_excuse_its_neighbour() {
        let tmp = workflows(&[(
            "ci.yml",
            "name: CI\n\njobs:\n  lint:\n    name: lint\n    continue-on-error: true\n  test:\n    name: test\n    runs-on: ubuntu-latest\n",
        )]);
        assert_eq!(workflow_job_names(tmp.path()), vec!["test"]);
    }

    #[test]
    fn no_workflows_is_no_checks_not_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(workflow_job_names(tmp.path()).is_empty());
    }

    #[test]
    fn a_slug_is_read_from_either_remote_form() {
        // Both forms appear in the wild; neither should be the one that works.
        for url in [
            "https://github.com/AleksaZCodes/fon.git",
            "git@github.com:AleksaZCodes/fon.git",
            "https://github.com/AleksaZCodes/fon",
        ] {
            let rest = url
                .strip_prefix("https://github.com/")
                .or_else(|| url.strip_prefix("git@github.com:"))
                .map(|r| r.trim_end_matches(".git").to_string());
            assert_eq!(rest.as_deref(), Some("AleksaZCodes/fon"), "{url}");
        }
    }
}
