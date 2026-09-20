//! `fid ai` — manage the advisory key, and hand the asking to something else.
//!
//! # `fid` still makes no network calls
//!
//! That posture is deliberate and this module preserves it. Everything remote
//! this binary does, it does by shelling out to a tool built for it — `git` for
//! remotes, `cargo`/`npm` for its own upgrades. Advisory review is the same
//! shape: `fid advise` spawns `fid-advise`, the Node CLI in
//! `packages/advisor`, which owns the HTTP and TLS.
//!
//! Two reasons, and the second is the load-bearing one:
//!
//! 1. Linking an HTTP and TLS stack into a binary that currently makes zero
//!    network calls is a large dependency for one advisory feature.
//! 2. The wire protocol is **already written and tested** in
//!    `packages/adapters/src/system-one.ts`. Writing it a second time in Rust
//!    would break principle 7 — nothing correct is solved twice — in the
//!    codebase that defines the principle.
//!
//! What `fid` does own is *writing the credentials file*, because writing a
//! config file is squarely its job and reaching for a second tool to do it
//! would be silly. No network is involved in storing a key.
//!
//! # Why the key is not in `fiducial.toml`
//!
//! `fiducial.toml` is committed. A secret in a committed file is a leaked
//! secret, and no amount of `.gitignore` discipline survives a `git add -A`.
//! The key goes to `~/.config/fiducial/credentials.toml` at mode 0600 —
//! per-developer, outside every repository, never in a diff.
//!
//! # Why this cannot affect a build
//!
//! A key unlocks advisory output and nothing else.
//! `crates/fiducial-cli/tests/determinism.rs` requires `fid derive` to be
//! byte-identical with and without one, and `fid dash`/`fid doctor` to report
//! identically. Read that file before adding anything here that a pipeline
//! could reach.

use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::Subcommand;

// ── Subcommand tree ───────────────────────────────────────────────────────────

/// Subcommands of `fid advise`.
///
/// Deliberately **not** `fid ai key`, which is what this was first written as.
/// `ai` had become ambiguous the moment a second kind of model arrived: it is
/// the name of the generative contract (`[adapters] ai`) *and* it was being used
/// as an umbrella for every model-based capability. A key that serves both
/// contracts filed under the name of one of them is a small lie that a reader
/// has to unlearn.
///
/// Hanging key management off `advise` instead names what the key actually
/// unlocks. It is honest in the other direction too: this key is for
/// developer-machine advisory tooling only. A product's runtime secrets go to
/// `wrangler secret put`, never here.
#[derive(Subcommand, Debug)]
pub enum AdviseCmd {
    /// Store, inspect, or remove the key that unlocks advisory review
    #[command(subcommand)]
    Key(KeyCmd),

    /// Report whether advisory checks are available, and what they are
    Status,
}

#[derive(Subcommand, Debug)]
pub enum KeyCmd {
    /// Store an advisory API key, read from stdin
    #[command(long_about = "\
Store an API key that unlocks advisory review. Read from stdin, never from an
argument: a key passed on the command line lands in your shell history and in
the process table, where any other process on the machine can read it.

  fid advise key set                      prompts, then reads one line
  pass show or/key | fid advise key set   pipe it from a secret manager

Stored at ~/.config/fiducial/credentials.toml, mode 0600 — per-developer and
outside every repository, because a secret in a committed file is a leaked
secret.

WHAT THIS UNLOCKS
  fid advise           review uncommitted changes for the rules code cannot check
  fid advise --facts   check new declarations against existing ones

WHAT THIS DOES NOT CHANGE
  Everything else. `fid derive`, `fid derive --check`, `fid doctor`, `fid dash`
  and the guard behave byte-for-byte identically with or without a key. That is
  not a promise in a doc comment — crates/fiducial-cli/tests/determinism.rs
  derives a product twice, with and without every key set, and requires the
  artifact trees to hash the same.

  A model is probabilistic. A gate whose verdict varies is not a gate, so AI in
  this platform advises and never derives.")]
    Set {
        /// Which vendor the key is for
        #[arg(
            value_name = "VENDOR",
            default_value = "openrouter",
            long_help = "\
`openrouter` (default) — the gateway. This is the same key `[adapters] ai =
\"openrouter\"` already uses, so one secret covers both contracts. Decisions go
to OpenRouter's decisions router, which preserves the typed-answer shape.

`typesafe` — straight to TypeSafe, no gateway. Choose it for a separate billing
relationship or a data-residency requirement."
        )]
        vendor: String,
    },

    /// Show which key is in use, redacted
    Show,

    /// Remove the stored credentials file
    Unset,
}

/// `~/.config/fiducial/credentials.toml`, honouring `XDG_CONFIG_HOME`.
///
/// Resolved from the environment rather than a crate so the test suite can
/// point `HOME` at a temporary directory and be sure a developer's real key
/// cannot leak into a test run.
fn credentials_path() -> Result<PathBuf> {
    let base = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config")
    } else {
        bail!("cannot locate a config directory: neither XDG_CONFIG_HOME nor HOME is set");
    };
    Ok(base.join("fiducial").join("credentials.toml"))
}

/// Parse `name = "value"` lines. One shape, one writer — no TOML crate needed
/// for three keys, and anything unparseable is skipped rather than fatal.
fn read_pairs(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        let Some(eq) = line.find('=') else { continue };
        let name = line[..eq].trim().to_string();
        let value = line[eq + 1..]
            .trim()
            .trim_matches('"')
            .trim_matches('\'')
            .to_string();
        if !name.is_empty() && !value.is_empty() {
            out.push((name, value));
        }
    }
    out
}

/// Show only enough of a key to recognise it. Never the whole thing: this
/// prints to a terminal that may be shared, recorded, or in a screenshot.
fn redact(key: &str) -> String {
    let n = key.chars().count();
    if n <= 10 {
        return "•".repeat(n);
    }
    let head: String = key.chars().take(6).collect();
    let tail: String = key.chars().skip(n - 4).collect();
    format!("{head}…{tail} ({n} chars)")
}

fn write_credentials(pairs: &[(String, String)]) -> Result<PathBuf> {
    let path = credentials_path()?;
    let dir = path.parent().expect("credentials path has a parent");
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;

    let mut body = String::from(
        "# fiducial credentials — written by `fid advise key set`.\n\
         #\n\
         # Per-developer and deliberately outside every repository: a secret in a\n\
         # committed file is a leaked secret. Nothing here changes what `fid derive`\n\
         # produces — a key unlocks advisory output only.\n\n",
    );
    for (name, value) in pairs {
        body.push_str(&format!("{name} = \"{value}\"\n"));
    }

    std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;

    // Owner-only. A key readable by every process on a shared machine is a key
    // that has been shared.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("restricting permissions on {}", path.display()))?;
    }

    Ok(path)
}

pub fn run(cmd: AdviseCmd) -> Result<()> {
    match cmd {
        AdviseCmd::Key(KeyCmd::Set { vendor }) => key_set(&vendor),
        AdviseCmd::Key(KeyCmd::Show) => key_show(),
        AdviseCmd::Key(KeyCmd::Unset) => key_unset(),
        AdviseCmd::Status => delegate(&["status"]),
    }
}

/// Read a key from stdin and store it.
///
/// Stdin, not an argument: a key on the command line lands in shell history and
/// in the process table, where it is readable by any other process on the
/// machine. Piping (`… | fid advise key set`) works for scripts.
fn key_set(vendor: &str) -> Result<()> {
    let name = match vendor {
        "openrouter" => "openrouter_api_key",
        "typesafe" => "typesafe_api_key",
        other => bail!(
            "unknown vendor `{other}`. Use `openrouter` (the gateway, and the \
             same key `[adapters] ai` uses) or `typesafe` (direct)."
        ),
    };

    if std::io::stdin().is_terminal() {
        println!("Paste the {vendor} API key, then press Enter.");
        println!("It is read from stdin so it does not land in your shell history.");
        print!("> ");
        std::io::stdout().flush().ok();
    }

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("reading the key from stdin")?;
    let key = input.trim().to_string();

    if key.is_empty() {
        bail!("no key on stdin — nothing written");
    }
    if key.contains(char::is_whitespace) {
        bail!("that value contains whitespace, so it is probably not just a key — nothing written");
    }

    let path = credentials_path()?;
    let mut pairs: Vec<(String, String)> = std::fs::read_to_string(&path)
        .map(|t| read_pairs(&t))
        .unwrap_or_default();
    pairs.retain(|(n, _)| n != name);
    pairs.push((name.to_string(), key.clone()));

    let written = write_credentials(&pairs)?;

    println!();
    println!("✦ stored {name} = {}", redact(&key));
    println!("  {}", written.display());
    println!();
    println!("  Unlocked:");
    println!("    fid advise            review uncommitted changes");
    println!("    fid advise --facts    check new declarations for duplicates");
    println!();
    println!("  Nothing else changes. `fid derive`, `fid doctor` and the guard are");
    println!("  byte-for-byte identical with or without this key — see");
    println!("  crates/fiducial-cli/tests/determinism.rs, which proves it.");
    Ok(())
}

fn key_show() -> Result<()> {
    let path = credentials_path()?;

    println!("✦ fid advise key");
    println!();

    // The environment wins at read time, so report it first or this command
    // would describe a key that is not the one being used.
    let mut from_env = false;
    for var in ["OPENROUTER_API_KEY", "TYPESAFE_API_KEY"] {
        if let Ok(value) = std::env::var(var) {
            if !value.is_empty() {
                println!(
                    "  {var}  {}  (environment — overrides the file)",
                    redact(&value)
                );
                from_env = true;
            }
        }
    }

    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let pairs = read_pairs(&text);
            if pairs.is_empty() {
                println!("  file     {} — no keys in it", path.display());
            } else {
                for (name, value) in &pairs {
                    println!("  {name}  {}", redact(value));
                }
                println!("  file     {}", path.display());
            }
        }
        Err(_) if from_env => {}
        Err(_) => {
            println!("  no key stored, and none in the environment");
            println!("  file     {} (does not exist)", path.display());
            println!();
            println!("  Set one:  fid advise key set");
            println!("  Advisory checks are off until you do; nothing else is affected.");
        }
    }
    Ok(())
}

fn key_unset() -> Result<()> {
    let path = credentials_path()?;
    if !path.exists() {
        println!("✦ nothing to remove — {} does not exist", path.display());
        return Ok(());
    }
    std::fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    println!("✦ removed {}", path.display());
    for var in ["OPENROUTER_API_KEY", "TYPESAFE_API_KEY"] {
        if std::env::var(var).is_ok_and(|v| !v.is_empty()) {
            println!("  note: {var} is still set in this environment and takes precedence.");
        }
    }
    Ok(())
}

/// Spawn `fid-advise`, inheriting stdio.
///
/// Resolution order mirrors how the platform is actually laid out: an explicit
/// override, then the repository's own copy (so working *on* the platform uses
/// the working tree, not a stale global install), then `node_modules/.bin`, then
/// PATH.
pub fn delegate(args: &[&str]) -> Result<()> {
    let cwd = std::env::current_dir().context("getting current directory")?;

    if let Ok(explicit) = std::env::var("FID_ADVISE_BIN") {
        // A `.js` path is run through `node` rather than exec'd. Pointing this at
        // a source file is the documented way to use a platform checkout from
        // another repository, and exec'ing it depends on an executable bit the
        // file has no reason to carry — which failed with a bare
        // "Permission denied" that named neither the cause nor the fix.
        if explicit.ends_with(".js") || explicit.ends_with(".mjs") {
            return spawn("node", &[explicit], args);
        }
        return spawn(&explicit, &[], args);
    }

    let root = crate::config::Config::find_root(&cwd).unwrap_or(cwd.clone());
    let candidates = [
        root.join("packages/advisor/src/cli.js"),
        root.join("node_modules/@fiducial/advisor/src/cli.js"),
    ];
    for path in candidates {
        if path.exists() {
            return spawn("node", &[path.to_string_lossy().to_string()], args);
        }
    }

    let local_bin = root.join("node_modules/.bin/fid-advise");
    if local_bin.exists() {
        return spawn(&local_bin.to_string_lossy(), &[], args);
    }

    spawn("fid-advise", &[], args)
}

fn spawn(program: &str, lead: &[String], args: &[&str]) -> Result<()> {
    let mut command = Command::new(program);
    command.args(lead).args(args);

    match command.status() {
        Ok(status) => {
            // The advisory tool decides its own exit code — it exits 0 even with
            // findings unless asked otherwise. Propagating it verbatim keeps that
            // contract in one place instead of two.
            if let Some(code) = status.code() {
                if code != 0 {
                    std::process::exit(code);
                }
            }
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            // Advisory tooling being absent is not a failure of whatever the
            // developer was actually doing.
            eprintln!("✦ fid advise — unavailable: could not run `{program}`.");
            eprintln!();
            eprintln!("  Advisory review runs in Node, because the decisions client is");
            eprintln!("  already written and tested in @fiducial/adapters and `fid` itself");
            eprintln!("  makes no network calls.");
            eprintln!();
            eprintln!("  Install it:   pnpm install   (in the platform repo)");
            eprintln!("  Or point at it: FID_ADVISE_BIN=/path/to/cli.js");
            Ok(())
        }
        Err(err) => Err(err).with_context(|| format!("running `{program}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_never_prints_the_middle_of_a_key() {
        let key = "sk-or-v1-abcdefghijklmnopqrstuvwxyz0123456789";
        let shown = redact(key);
        assert!(shown.starts_with("sk-or-"));
        assert!(!shown.contains("lmnopqrst"));
        assert!(shown.contains("6789"));
    }

    #[test]
    fn a_short_secret_is_fully_masked() {
        // Below the point where a prefix and suffix would be most of the value.
        assert_eq!(redact("abc"), "•••");
    }

    #[test]
    fn pairs_skip_comments_sections_and_junk() {
        let text = "# a comment\n[section]\nopenrouter_api_key = \"abc\"\nbroken line\n";
        let pairs = read_pairs(text);
        assert_eq!(
            pairs,
            vec![("openrouter_api_key".to_string(), "abc".to_string())]
        );
    }
}
