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
enum Step {
    /// Run `fid …` and record what it prints.
    Capture {
        file: &'static str,
        args: &'static [&'static str],
    },
    /// Run `fid …` only to advance the product's state; record nothing.
    Setup { args: &'static [&'static str] },
    /// Edit a declaration in place, the way a reader following the guide does.
    ///
    /// The interesting captures are the *failing* ones — an artifact that no
    /// longer matches its declaration is the guarantee this platform sells, and
    /// it cannot be reached by running `fid` commands alone. Without this the
    /// stale-output block in `docs/guides/first-product.md` had to be written
    /// from memory, and it was wrong: it named `enclosure/case-base.stl` and a
    /// wording the binary has never printed.
    Edit {
        file: &'static str,
        from: &'static str,
        to: &'static str,
    },
}

const fn capture(file: &'static str, args: &'static [&'static str]) -> Step {
    Step::Capture { file, args }
}

const fn setup(args: &'static [&'static str]) -> Step {
    Step::Setup { args }
}

const fn edit(file: &'static str, from: &'static str, to: &'static str) -> Step {
    Step::Edit { file, from, to }
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
        // The guarantee, shown failing. Last, because it leaves the product
        // deliberately stale and every capture above it expects fresh.
        edit(
            "board/board.interface.json",
            "\"width_mm\": 100.0",
            "\"width_mm\": 120.0",
        ),
        capture("fid-derive-check-stale.txt", &["derive", "--check"]),
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
        let (file, args) = match step {
            Step::Capture { file, args } => (Some(file), args),
            Step::Setup { args } => (None, args),
            Step::Edit { file, from, to } => {
                let path = product.join(file);
                let before = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("reading {file}: {e}"));
                // A silent no-op here would make the next capture record the
                // fresh output under a name promising the stale one.
                assert!(
                    before.contains(from),
                    "{file} does not contain `{from}` — the declaration moved \
                     and this edit no longer does anything"
                );
                std::fs::write(&path, before.replace(from, to))
                    .unwrap_or_else(|e| panic!("writing {file}: {e}"));
                continue;
            }
        };

        // `fid new` runs in the scratch directory; everything else runs inside
        // the product it created.
        let cwd = if args[0] == "new" { &scratch } else { &product };

        let output = Command::new(fid())
            .args(args)
            .current_dir(cwd)
            .env("NO_COLOR", "1")
            // A capture must be reproducible on another machine. `anyhow`
            // prints a backtrace when `RUST_BACKTRACE` is set, so a developer
            // who exports it — and this repository's own CI container does —
            // would regenerate every failing capture with fifty lines of
            // `/rustc/<hash>/library/…` paths in it. Harmless until a capture
            // was allowed to fail; the stale-check capture is the first.
            .env("RUST_BACKTRACE", "0")
            .output()
            .unwrap_or_else(|e| panic!("running `fid {}`: {e}", args.join(" ")));

        let Some(name) = file else {
            // A setup step only has to succeed; nothing is recorded from it.
            assert!(
                output.status.success(),
                "setup step `fid {}` failed:\n{}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            );
            continue;
        };

        let mut body = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            body.push_str(&stderr);
        }

        let header = format!("$ fid {}\n", args.join(" "));
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
    // The same document list the inliner walks, so a capture shown only in
    // `README.md` counts as referenced rather than reading as dead weight.
    let mut prose = String::new();
    for path in documents_with_captures(&root) {
        prose.push_str(&std::fs::read_to_string(path).unwrap_or_default());
    }

    let mut unreferenced: Vec<String> = Vec::new();
    for step in steps() {
        let Step::Capture { file: name, .. } = step else {
            continue;
        };
        if !prose.contains(name) {
            unreferenced.push(format!("  {name} — generated but no guide shows it"));
        }
    }

    assert!(
        unreferenced.is_empty(),
        "every declared capture is shown in a document, or it is dead weight:\n\n{}\n",
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

/// Every document that embeds terminal output, wherever it lives.
///
/// `README.md` is on this list because it is the most-read page in the
/// repository and was the least gated: it showed `fid` output that no test had
/// ever compared against the binary. A capture marker is worth nothing if the
/// file it is in is not scanned.
fn documents_with_captures(root: &Path) -> Vec<PathBuf> {
    let mut out = vec![root.join("README.md")];
    if let Ok(entries) = std::fs::read_dir(root.join("docs/guides")) {
        let mut guides: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .collect();
        guides.sort();
        out.extend(guides);
    }
    out
}

/// Every capture block in every document matches the capture it names.
#[test]
fn documents_embed_the_current_captures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf();
    let captures = render();

    let writing = std::env::var("FIDUCIAL_WRITE_CAPTURES").is_ok();
    let mut stale: Vec<String> = Vec::new();

    for path in documents_with_captures(&root) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !text.contains(OPEN) {
            continue;
        }

        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let rendered = match inline_captures(&text, &captures) {
            Ok(r) => r,
            Err(e) => {
                stale.push(format!("  {rel} — {e}"));
                continue;
            }
        };

        if writing {
            if rendered != text {
                std::fs::write(&path, &rendered).expect("write document");
                eprintln!("inlined captures into {rel}");
            }
        } else if rendered != text {
            stale.push(format!("  {rel} — an embedded capture is out of date"));
        }
    }

    assert!(
        stale.is_empty(),
        "\nA document shows terminal output the CLI no longer produces.\n\n{}\n\n\
         Regenerate with:\n  \
         FIDUCIAL_WRITE_CAPTURES=1 cargo test -p fiducial-cli --test captures\n",
        stale.join("\n")
    );
}
