//! Whether the gate still runs.
//!
//! Every other check in `fid derive --check` assumes it is run. A product
//! whose CI step was deleted, commented out, or told to ignore its result is
//! green on every mistake at once, and nothing inside the product says so: the
//! paper's independent set got exactly this past 0.9.12 (I10).
//!
//! So the gate checks for itself. When the repository has workflows, one live
//! step must run `fid derive --check` in this product's directory and let its
//! failure fail the job.

use std::path::{Path, PathBuf};

const WORKFLOWS: &str = ".github/workflows";
const SCAFFOLDED: &str = ".github/workflows/ci.yml";

/// How a step that runs the gate could still let a red result through.
const SILENCED_RUN: [&str; 4] = ["|| true", "|| :", "|| exit 0", "; true"];

#[derive(Debug, PartialEq)]
pub struct Step {
    pub file: String,
    pub working_dir: String,
    pub silenced: Option<String>,
}

/// Problems with how CI runs the gate for the product at `root`, as lines for
/// `fid derive --check` and `fid doctor`. Empty when a step runs it, or when
/// the repository has no workflows and never had the scaffolded one.
pub fn problems(root: &Path, scaffolded_tracked: bool) -> Vec<String> {
    let repo = repo_root(root);
    let product = rel(&repo, root);
    let dir = repo.join(WORKFLOWS);
    let files = workflow_files(&dir);
    if files.is_empty() {
        if scaffolded_tracked {
            return vec![format!(
                "  {SCAFFOLDED}: deleted — nothing runs `fid derive --check` on a change, so \
                 every other check here is advisory. Restore it with `fid upgrade`."
            )];
        }
        return Vec::new();
    }
    let steps: Vec<Step> = files
        .iter()
        .filter_map(|p| {
            let text = std::fs::read_to_string(p).ok()?;
            let name = p.file_name()?.to_string_lossy().to_string();
            Some(steps_running_check(&name, &text))
        })
        .flatten()
        .collect();
    let here: Vec<&Step> = steps.iter().filter(|s| s.working_dir == product).collect();
    if here.is_empty() {
        let shown = if product == "." {
            "the repository root".to_string()
        } else {
            format!("`{product}`")
        };
        let elsewhere = if steps.is_empty() {
            String::new()
        } else {
            format!(
                " (it runs in {})",
                steps
                    .iter()
                    .map(|s| format!("`{}`", s.working_dir))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        return vec![format!(
            "  {WORKFLOWS}/: no step runs `fid derive --check` in {shown}{elsewhere} — \
             without it every other check here is advisory. Add the step back \
             (`fid upgrade` restores the scaffolded one)."
        )];
    }
    if here.iter().all(|s| s.silenced.is_some()) {
        return here
            .iter()
            .map(|s| {
                format!(
                    "  {WORKFLOWS}/{}: runs `fid derive --check` but {} — a red result \
                     would not fail the build. Remove it.",
                    s.file,
                    s.silenced.as_deref().unwrap_or_default()
                )
            })
            .collect();
    }
    Vec::new()
}

/// Every step in one workflow whose `run` invokes `derive --check`, with the
/// directory it runs in relative to the repository root.
pub fn steps_running_check(file: &str, text: &str) -> Vec<Step> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut job_indent: Option<usize> = None;
    let mut job_default = String::from(".");
    let mut in_defaults = false;
    let mut in_jobs = false;
    let mut i = 0;
    while i < lines.len() {
        let line = strip_comment(lines[i]);
        let ind = indent_of(line);
        let t = line.trim();
        if t.is_empty() {
            i += 1;
            continue;
        }
        if ind == 0 {
            in_jobs = t == "jobs:";
            job_indent = None;
            i += 1;
            continue;
        }
        if in_jobs && (job_indent.is_none() || ind <= job_indent.unwrap_or(0)) && t.ends_with(':') {
            job_indent = Some(ind);
            job_default = ".".into();
            in_defaults = false;
            i += 1;
            continue;
        }
        if t == "defaults:" {
            in_defaults = true;
        } else if in_defaults && t.starts_with("working-directory:") {
            job_default = normalise(value(t, "working-directory:"));
        } else if t.starts_with("steps:") {
            in_defaults = false;
        }
        if t.starts_with("- ") || t == "-" {
            let step_ind = ind;
            let mut j = i + 1;
            while j < lines.len() {
                let l = strip_comment(lines[j]);
                if !l.trim().is_empty() && indent_of(l) <= step_ind {
                    break;
                }
                j += 1;
            }
            let block: Vec<String> = std::iter::once(t.trim_start_matches('-').trim().to_string())
                .chain(
                    lines[i + 1..j]
                        .iter()
                        .map(|l| strip_comment(l).trim().to_string()),
                )
                .collect();
            if let Some(step) = step_from(file, &block, &job_default) {
                out.push(step);
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

fn step_from(file: &str, block: &[String], job_default: &str) -> Option<Step> {
    let runs = block.iter().position(|l| l.starts_with("run:")).map(|k| {
        let first = value(&block[k], "run:").to_string();
        let rest = if first.starts_with('|') || first.starts_with('>') {
            block[k + 1..]
                .iter()
                .take_while(|l| !is_key(l))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            vec![first]
        };
        rest
    })?;
    let line = runs.iter().find(|l| l.contains("derive --check"))?;
    let mut wd = block
        .iter()
        .find(|l| l.starts_with("working-directory:"))
        .map(|l| normalise(value(l, "working-directory:")))
        .unwrap_or_else(|| job_default.to_string());
    if let Some(dir) = line.strip_prefix("cd ").and_then(|r| r.split("&&").next()) {
        wd = join(&wd, dir.trim());
    }
    let silenced = SILENCED_RUN
        .iter()
        .find(|s| line.contains(**s))
        .map(|s| format!("ends in `{s}`"))
        .or_else(|| {
            block
                .iter()
                .find(|l| {
                    let l = l.replace(' ', "");
                    l == "continue-on-error:true" || l == "if:false" || l == "if:${{false}}"
                })
                .map(|l| format!("sets `{l}`"))
        });
    Some(Step {
        file: file.to_string(),
        working_dir: wd,
        silenced,
    })
}

fn is_key(l: &str) -> bool {
    let key = l.split(':').next().unwrap_or("");
    l.contains(':')
        && !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn value<'a>(line: &'a str, key: &str) -> &'a str {
    line.trim()
        .trim_start_matches(key)
        .trim()
        .trim_matches(['"', '\''])
}

fn normalise(dir: &str) -> String {
    let d = dir.trim().trim_start_matches("./").trim_end_matches('/');
    if d.is_empty() || d == "." || d == "${{ github.workspace }}" {
        ".".into()
    } else {
        d.to_string()
    }
}

fn join(base: &str, dir: &str) -> String {
    let d = normalise(dir);
    if base == "." || d.starts_with('/') {
        d
    } else if d == "." {
        base.to_string()
    } else {
        format!("{base}/{d}")
    }
}

fn workflow_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .collect();
    files.sort();
    files
}

/// Where GitHub looks for workflows: the repository root, the nearest
/// ancestor holding `.git`. A `.github/` inside a product nested in a larger
/// repository is never run, so it does not count. Outside any repository,
/// the product root.
fn repo_root(root: &Path) -> PathBuf {
    let abs = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    abs.ancestors()
        .find(|a| a.join(".git").exists())
        .map(Path::to_path_buf)
        .unwrap_or(abs)
}

fn rel(repo: &Path, root: &Path) -> String {
    let abs = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let r = abs
        .strip_prefix(repo)
        .unwrap_or(Path::new(""))
        .to_string_lossy()
        .replace('\\', "/");
    if r.is_empty() {
        ".".into()
    } else {
        r
    }
}

fn strip_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut quote: Option<u8> = None;
    for (i, &b) in bytes.iter().enumerate() {
        match quote {
            Some(q) if b == q => quote = None,
            Some(_) => {}
            None if b == b'"' || b == b'\'' => quote = Some(b),
            None if b == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) => {
                return &line[..i];
            }
            None => {}
        }
    }
    line
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCAFFOLD: &str = "name: CI\non:\n  push:\njobs:\n  fiducial:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      # Fails when stale.\n      - name: fid derive --check\n        run: fid derive --check\n";

    #[test]
    fn the_scaffolded_step_is_found_at_the_root() {
        let s = steps_running_check("ci.yml", SCAFFOLD);
        assert_eq!(
            s,
            vec![Step {
                file: "ci.yml".into(),
                working_dir: ".".into(),
                silenced: None
            }]
        );
    }

    #[test]
    fn a_job_default_directory_is_where_the_step_runs() {
        let y = "jobs:\n  example:\n    defaults:\n      run:\n        working-directory: examples/stick\n    steps:\n      - name: build\n        working-directory: .\n        run: cargo build\n      - name: gate\n        run: fid derive --check\n";
        let s = steps_running_check("ci.yml", y);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].working_dir, "examples/stick");
    }

    #[test]
    fn a_deleted_or_commented_step_is_not_a_gate() {
        let deleted = SCAFFOLD.replace(
            "      - name: fid derive --check\n        run: fid derive --check\n",
            "",
        );
        assert!(steps_running_check("ci.yml", &deleted).is_empty());
        let commented = SCAFFOLD.replace(
            "        run: fid derive --check",
            "        # run: fid derive --check",
        );
        assert!(steps_running_check("ci.yml", &commented).is_empty());
    }

    #[test]
    fn a_step_told_to_ignore_its_result_is_named() {
        let t = SCAFFOLD.replace("run: fid derive --check", "run: fid derive --check || true");
        assert_eq!(
            steps_running_check("ci.yml", &t)[0].silenced.as_deref(),
            Some("ends in `|| true`")
        );
        let c = SCAFFOLD.replace(
            "        run: fid derive --check\n",
            "        continue-on-error: true\n        run: fid derive --check\n",
        );
        assert!(steps_running_check("ci.yml", &c)[0].silenced.is_some());
    }

    #[test]
    fn a_multiline_run_with_cd_counts_in_that_directory() {
        let y = "jobs:\n  a:\n    steps:\n      - run: |\n          cd product && fid derive --check\n        name: gate\n";
        assert_eq!(steps_running_check("ci.yml", y)[0].working_dir, "product");
    }

    #[test]
    fn problems_name_a_missing_gate_and_pass_a_present_one() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        assert!(
            problems(root, false).is_empty(),
            "no CI and none scaffolded is not this check's call"
        );
        assert_eq!(
            problems(root, true).len(),
            1,
            "the scaffolded workflow deleted"
        );
        std::fs::create_dir_all(root.join(WORKFLOWS)).unwrap();
        std::fs::write(root.join(SCAFFOLDED), SCAFFOLD).unwrap();
        assert!(problems(root, true).is_empty());
        let gone = SCAFFOLD.replace(
            "        run: fid derive --check\n",
            "        run: echo skipped\n",
        );
        std::fs::write(root.join(SCAFFOLDED), gone).unwrap();
        let p = problems(root, true);
        assert!(
            p[0].contains("no step runs `fid derive --check` in the repository root"),
            "{p:?}"
        );
    }

    #[test]
    fn a_workflow_nested_in_a_product_is_not_one_github_runs() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join(WORKFLOWS)).unwrap();
        std::fs::write(
            repo.join(SCAFFOLDED),
            "jobs:\n  a:\n    steps:\n      - run: cargo test\n",
        )
        .unwrap();
        let product = repo.join("examples/p");
        std::fs::create_dir_all(product.join(WORKFLOWS)).unwrap();
        std::fs::write(product.join(SCAFFOLDED), SCAFFOLD).unwrap();
        let p = problems(&product, true);
        assert!(p[0].contains("in `examples/p`"), "{p:?}");
        let gated = "jobs:\n  a:\n    defaults:\n      run:\n        working-directory: examples/p\n    steps:\n      - run: fid derive --check\n";
        std::fs::write(repo.join(SCAFFOLDED), gated).unwrap();
        assert!(problems(&product, true).is_empty());
    }
}
