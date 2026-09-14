//! Terminal captures for the documentation — generated, committed, CI-checked.
//!
//! # Why these are generated
//!
//! The guides in `docs/guides/` show what `fid` actually prints. A hand-pasted
//! terminal block is a derived artifact maintained by memory: it is correct on
//! the day it is pasted and drifts silently from then on, and the reader has no
//! way to tell. That is the exact failure this platform exists to delete, so
//! writing one into our own documentation would be indefensible.
//!
//! So the captures are **derived from the real binary**. This test runs a
//! declared list of commands against a freshly scaffolded product, records what
//! they print, and asserts the committed files still match.
//!
//! # Regenerating
//!
//! ```sh
//! FIDUCIAL_WRITE_CAPTURES=1 cargo test -p fiducial-cli --test captures
//! ```
//!
//! CI runs it **without** that variable, so a change to `fid`'s output that
//! skipped regeneration fails the build rather than reaching `main` as a guide
//! that lies. This is the same freshness gate `docs/protocol/vectors.json` uses.
//!
//! # What is normalized, and why that is safe
//!
//! A capture must be reproducible on another machine and on another day, so
//! three things are replaced with stable placeholders: the temporary directory
//! path, the git commit hash, and the platform version. Everything else —
//! wording, alignment, section order, findings — is byte-compared. Those are
//! the parts a reader relies on, and they are the parts that drift.

use std::path::{Path, PathBuf};
use std::process::Command;

/// One step in the documented walkthrough.
///
/// Steps run **in order against the same product**, so the captures show the
/// state a reader actually reaches at that point in the guide. Some steps exist
/// only to advance that state — `fid add eda` installs the pipelines that make
/// `fid graph` show anything — and those have no `file`.
///
/// This matters more than it looks. The first version of this harness ran every
/// command against a bare scaffold, so `fid graph` was captured as "no pipelines
/// declared" and embedded into a guide section that comes *after* the pipelines
/// are installed. The capture was real output, faithfully generated, and
/// completely misleading — which is the failure mode generated documentation is
/// supposed to remove, not introduce.
struct Step {
    /// Where the output lands, or `None` for a step that only advances state.
    file: Option<&'static str>,
    args: &'static [&'static str],
}

const fn capture(file: &'static str, args: &'static [&'static str]) -> Step {
    Step {
        file: Some(file),
        args,
    }
}

const fn setup(args: &'static [&'static str]) -> Step {
    Step { file: None, args }
}

/// The single declaration of what the documentation shows, in the order a
/// reader encounters it.
fn steps() -> Vec<Step> {
    vec![
        capture("fid-new.txt", &["new", "demo-product"]),
        capture("fid-doctor.txt", &["doctor"]),
        capture("fid-dash.txt", &["dash"]),
        capture("fid-dash-ci.txt", &["dash", "--section", "ci"]),
        // From here the guide has a board, so the captures must too.
        setup(&["add", "eda"]),
        capture("fid-graph.txt", &["graph"]),
        setup(&["derive"]),
        capture("fid-derive-check.txt", &["derive", "--check"]),
        capture("fid-release-status.txt", &["release", "status"]),
    ]
}

fn fid() -> PathBuf {
    let mut path = std::env::current_exe().expect("test binary path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("fid")
}

fn captures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("docs/captures")
}

/// Replace everything that legitimately differs between two runs.
///
/// Anything NOT normalized here is compared byte for byte — which is the point.
fn normalize(raw: &str, scratch: &Path, product: &Path) -> String {
    let mut text = raw.replace(&product.display().to_string(), "/home/you/dev/demo-product");
    text = text.replace(&scratch.display().to_string(), "/home/you/dev");

    let normalized: Vec<String> = text
        .lines()
        .map(|line| {
            // `head   a1b2c3d  chore: …` — the hash changes every run.
            if let Some(rest) = line.trim_start().strip_prefix("head") {
                let indent = &line[..line.len() - line.trim_start().len()];
                let rest = rest.trim_start();
                if let Some((hash, message)) = rest.split_once(char::is_whitespace) {
                    if hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                        return format!("{indent}head           0000000  {}", message.trim_start());
                    }
                }
            }
            // The platform version moves with every release.
            if line.contains(env!("CARGO_PKG_VERSION")) {
                return line.replace(env!("CARGO_PKG_VERSION"), "X.Y.Z");
            }
            line.to_string()
        })
        .collect();

    let mut out = normalized.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Run every declared command against one freshly scaffolded product.
fn render() -> Vec<(String, String)> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let scratch = tmp.path().canonicalize().expect("canonical tempdir");
    let product = scratch.join("demo-product");

    let mut rendered = Vec::new();

    for step in steps() {
        // `fid new` runs in the scratch directory; everything else runs inside
        // the product it created.
        let cwd = if step.args[0] == "new" {
            &scratch
        } else {
            &product
        };

        let output = Command::new(fid())
            .args(step.args)
            .current_dir(cwd)
            .env("NO_COLOR", "1")
            .output()
            .unwrap_or_else(|e| panic!("running `fid {}`: {e}", step.args.join(" ")));

        let Some(name) = step.file else {
            // A setup step only has to succeed; nothing is recorded from it.
            assert!(
                output.status.success(),
                "setup step `fid {}` failed:\n{}",
                step.args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            );
            continue;
        };

        let mut body = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            body.push_str(&stderr);
        }

        let header = format!("$ fid {}\n", step.args.join(" "));
        rendered.push((
            name.to_string(),
            format!("{header}{}", normalize(&body, &scratch, &product)),
        ));
    }

    rendered
}

/// The committed captures match what the binary prints today.
#[test]
fn captures_are_fresh() {
    let dir = captures_dir();
    let rendered = render();

    if std::env::var("FIDUCIAL_WRITE_CAPTURES").is_ok() {
        std::fs::create_dir_all(&dir).expect("mkdir docs/captures");
        for (name, body) in &rendered {
            std::fs::write(dir.join(name), body).expect("write capture");
        }
        eprintln!("wrote {} captures to {}", rendered.len(), dir.display());
        return;
    }

    let mut stale: Vec<String> = Vec::new();

    for (name, body) in &rendered {
        let path = dir.join(name);
        let committed = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => {
                stale.push(format!("  docs/captures/{name} — missing"));
                continue;
            }
        };
        if committed != *body {
            stale.push(format!(
                "  docs/captures/{name} — differs from what `fid` prints now"
            ));
        }
    }

    assert!(
        stale.is_empty(),
        "\nThe committed terminal captures no longer match the binary, so the \
         guides in docs/guides/ are showing output the CLI does not produce.\n\n{}\n\n\
         Regenerate with:\n  \
         FIDUCIAL_WRITE_CAPTURES=1 cargo test -p fiducial-cli --test captures\n\n\
         Then read the diff before committing — a changed capture is a changed \
         user experience, and it is worth a look.\n",
        stale.join("\n")
    );
}

/// Every capture the documentation references exists, and every capture that
/// exists is referenced.
///
/// An orphaned capture is dead weight; a referenced-but-missing one renders as a
/// broken include. Both are silent without this.
#[test]
fn captures_and_their_references_agree() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();
    let guides = root.join("docs/guides");

    let mut prose = String::new();
    if let Ok(entries) = std::fs::read_dir(&guides) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|e| e == "md") {
                prose.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
            }
        }
    }

    let mut unreferenced: Vec<String> = Vec::new();
    for step in steps() {
        let Some(name) = step.file else { continue };
        if !prose.contains(name) {
            unreferenced.push(format!("  {name} — generated but no guide shows it"));
        }
    }

    assert!(
        unreferenced.is_empty(),
        "every declared capture is shown in a guide, or it is dead weight:\n\n{}\n",
        unreferenced.join("\n")
    );
}

// ── Inlining captures into the guides ────────────────────────────────────────

/// The guides embed capture output between HTML markers:
///
/// ```markdown
/// <!-- capture: fid-dash.txt -->
/// ```text
/// … the capture, verbatim …
/// ```
/// <!-- /capture -->
/// ```
///
/// The block between the markers is **derived** from `docs/captures/`. It is
/// embedded rather than linked because GitHub renders neither transclusion nor
/// snippet syntax, and a guide whose terminal output is a link nobody clicks is
/// a guide that shows nothing.
///
/// So the capture file is the declaration and the guide block is the
/// derivation, which is the same relationship every other artifact in this
/// repository has. Regenerated and checked by the same environment variable.
const OPEN: &str = "<!-- capture: ";
const CLOSE: &str = "<!-- /capture -->";

/// Rewrite every capture block in `markdown`, returning the new text.
///
/// Takes the rendered captures **in memory** rather than reading
/// `docs/captures/`. The first version read the files, which made this test
/// depend on another test having already written them — and cargo runs tests in
/// parallel, so it read whatever was on disk at the time. Both tests now derive
/// from `render()`, so there is one source and no ordering between them.
fn inline_captures(markdown: &str, captures: &[(String, String)]) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = markdown;

    while let Some(start) = rest.find(OPEN) {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + OPEN.len()..];

        let name_end = after_open
            .find(" -->")
            .ok_or_else(|| format!("unterminated `{OPEN}` marker"))?;
        let name = after_open[..name_end].trim().to_string();

        let body_start = start + OPEN.len() + name_end + " -->".len();
        let close_at = rest[body_start..]
            .find(CLOSE)
            .ok_or_else(|| format!("`{name}` block has no `{CLOSE}`"))?;

        let capture = captures
            .iter()
            .find(|(file, _)| *file == name)
            .map(|(_, body)| body.clone())
            .ok_or_else(|| format!("`{name}` is not a declared capture"))?;

        out.push_str(OPEN);
        out.push_str(&name);
        out.push_str(" -->\n\n```text\n");
        out.push_str(capture.trim_end());
        out.push_str("\n```\n\n");
        out.push_str(CLOSE);

        rest = &rest[body_start + close_at + CLOSE.len()..];
    }

    out.push_str(rest);
    Ok(out)
}

/// Every capture block in every guide matches the capture it names.
#[test]
fn guides_embed_the_current_captures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();
    let guides = root.join("docs/guides");
    let captures = render();

    let writing = std::env::var("FIDUCIAL_WRITE_CAPTURES").is_ok();
    let mut stale: Vec<String> = Vec::new();

    let entries = match std::fs::read_dir(&guides) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("guide is readable");
        if !text.contains(OPEN) {
            continue;
        }

        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let rendered = match inline_captures(&text, &captures) {
            Ok(r) => r,
            Err(e) => {
                stale.push(format!("  docs/guides/{name} — {e}"));
                continue;
            }
        };

        if writing {
            if rendered != text {
                std::fs::write(&path, &rendered).expect("write guide");
                eprintln!("inlined captures into docs/guides/{name}");
            }
        } else if rendered != text {
            stale.push(format!(
                "  docs/guides/{name} — an embedded capture is out of date"
            ));
        }
    }

    assert!(
        stale.is_empty(),
        "\nA guide shows terminal output the CLI no longer produces.\n\n{}\n\n\
         Regenerate with:\n  \
         FIDUCIAL_WRITE_CAPTURES=1 cargo test -p fiducial-cli --test captures\n",
        stale.join("\n")
    );
}
