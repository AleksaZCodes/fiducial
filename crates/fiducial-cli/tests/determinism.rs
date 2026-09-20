//! `fid derive` is byte-identical whether or not an AI key is reachable.
//!
//! # Why this file exists
//!
//! The platform can reach a decision model (`systemOne`, see
//! `packages/adapters/src/system-one.ts`). A model is probabilistic: the same
//! question over the same state may come back with a different distribution,
//! and a calibrated 0.83 is not a promise of 0.83 next time.
//!
//! Derivation cannot be probabilistic. `fid derive --check` gates CI on
//! artifact freshness, so if any model output reached an artifact, two runs
//! would differ, `--check` would fail on a clean tree, and the gate that makes
//! every derived artifact trustworthy would start crying wolf. The failure
//! would also be *intermittent*, which is the worst kind: a gate that fails
//! one commit in twenty gets disabled rather than diagnosed.
//!
//! So the rule for this platform is **AI advises, it never derives.** A key
//! unlocks additional advisory output that a human or an agent reads; it never
//! changes an artifact, a gate's verdict, or a derived fact.
//!
//! That rule is worth *proving* rather than documenting, because it is exactly
//! the kind of invariant that erodes one convenient call at a time. Each test
//! here runs the same derivation twice — once with nothing set, once with every
//! key the platform knows how to read — and requires the artifact tree to hash
//! identically.
//!
//! A failure here does not mean the model behaved badly. It means something in
//! the derive path started *asking*, and the fix is to move that call to an
//! advisory surface, not to loosen this test.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fid() -> PathBuf {
    let mut path = std::env::current_exe().expect("test binary path");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.join("fid")
}

/// Run `fid` with an explicitly *empty* AI environment.
///
/// Every key is cleared rather than merely left unset: the developer running
/// this suite may well have `OPENROUTER_API_KEY` exported, and a test whose
/// meaning depends on the operator's shell is not a test. `HOME` is redirected
/// at the caller's choosing so a real `~/.config/fiducial/credentials.toml`
/// cannot leak in either.
fn run_without_keys(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(fid())
        .args(args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .env("HOME", home)
        .env_remove("OPENROUTER_API_KEY")
        .env_remove("TYPESAFE_API_KEY")
        .env_remove("FIDUCIAL_ADVISE")
        .output()
        .expect("running fid")
}

/// Run `fid` with every AI key set to a plausible-looking value.
///
/// The values are syntactically credible but not real. That is deliberate: if
/// some code path in derive were to *use* a key, a well-formed one takes it
/// further down that path — and therefore closer to producing a difference this
/// test can catch — than an obviously empty string would.
///
/// The two values are **assembled at runtime rather than written as literals**,
/// and that is not style. A literal shaped like a real key trips GitHub's push
/// protection, which cannot tell a placeholder from the real thing and should
/// not try — it blocked this very file. Assembling the string keeps the shape
/// the test wants without putting a key-shaped literal in the repository. Do not
/// "simplify" these back into literals; the push will be rejected again.
fn run_with_keys(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    let openrouter = format!("sk-or-{}-{}", "v1", "0".repeat(64));
    let typesafe = format!("ts-{}", "0".repeat(40));

    Command::new(fid())
        .args(args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .env("HOME", home)
        .env("OPENROUTER_API_KEY", openrouter)
        .env("TYPESAFE_API_KEY", typesafe)
        .env("FIDUCIAL_ADVISE", "1")
        .output()
        .expect("running fid")
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// Every tracked file under `root`, mapped to the SHA-256 of its bytes.
///
/// Hashes rather than contents so a failure prints a short diff of *which*
/// paths changed instead of every byte of every artifact. `.git` and
/// `node_modules` are skipped as churn unrelated to derivation; `fiducial.lock`
/// is kept, because a lock entry is itself a derived fact and a model reaching
/// it would be exactly the bug this file hunts.
fn artifact_hashes(root: &Path) -> BTreeMap<String, String> {
    use sha2::{Digest, Sha256};

    let mut out = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != "node_modules" && name != "target"
        })
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = std::fs::read(entry.path()).expect("reading artifact");
        out.insert(rel, hex::encode(Sha256::digest(&bytes)));
    }
    out
}

/// Report the paths whose hash differs between two runs.
fn differing(a: &BTreeMap<String, String>, b: &BTreeMap<String, String>) -> Vec<String> {
    let mut diffs = Vec::new();
    for (path, hash) in a {
        match b.get(path) {
            None => diffs.push(format!("  {path} — present without keys, absent with them")),
            Some(other) if other != hash => {
                diffs.push(format!("  {path} — contents differ between runs"))
            }
            Some(_) => {}
        }
    }
    for path in b.keys() {
        if !a.contains_key(path) {
            diffs.push(format!("  {path} — appeared only when keys were set"));
        }
    }
    diffs
}

/// A scaffolded product with several capabilities and a declared decision model.
///
/// `systemOne = "openrouter"` is selected on purpose: the point is not that a
/// product which never mentions AI derives deterministically — that is trivial
/// — but that one which *has* the contract wired still does.
fn product(tmp: &Path, home: &Path) -> PathBuf {
    assert!(
        run_without_keys(tmp, home, &["new", "p"]).status.success(),
        "fid new"
    );
    let root = tmp.join("p");
    for cap in ["adapters", "brand", "legal"] {
        let out = run_without_keys(&root, home, &["add", cap]);
        assert!(out.status.success(), "fid add {cap}: {}", text(&out));
    }

    let config = root.join("fiducial.toml");
    let existing = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        format!(
            "{existing}\n[adapters]\nai = \"openrouter\"\nsystemOne = \"openrouter\"\n\
             storage = \"r2\"\nqueue = \"cloudflare-queues\"\n\
             \n[ai]\nmodel = \"anthropic/claude-opus-5\"\n\
             \n[systemOne]\nmodel = \"typesafe/jev-1.13\"\n"
        ),
    )
    .unwrap();
    root
}

#[test]
fn derive_is_byte_identical_with_and_without_ai_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let root = product(tmp.path(), home.path());

    let first = run_without_keys(&root, home.path(), &["derive"]);
    assert!(
        first.status.success(),
        "derive without keys: {}",
        text(&first)
    );
    let without = artifact_hashes(&root);

    let second = run_with_keys(&root, home.path(), &["derive"]);
    assert!(
        second.status.success(),
        "derive with keys: {}",
        text(&second)
    );
    let with = artifact_hashes(&root);

    let diffs = differing(&without, &with);
    assert!(
        diffs.is_empty(),
        "`fid derive` is not deterministic across AI-key presence.\n\n\
         AI advises; it never derives. Something in the derive path is reading a \
         key or asking a model, which makes `fid derive --check` fail \
         intermittently on a clean tree.\n\n\
         Paths that changed:\n{}\n\n\
         Fix by moving that call to an advisory surface (one that exits 0 and \
         writes no artifact), not by relaxing this test.",
        diffs.join("\n")
    );
    assert!(
        !without.is_empty(),
        "the fixture derived no artifacts at all, so this test proved nothing"
    );
}

#[test]
fn derive_check_passes_with_keys_after_deriving_without_them() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let root = product(tmp.path(), home.path());

    assert!(run_without_keys(&root, home.path(), &["derive"])
        .status
        .success());

    // The freshness gate is the thing a model in the derive path would actually
    // break, and it breaks it in CI rather than locally — so assert on the gate
    // itself, not only on the bytes.
    let check = run_with_keys(&root, home.path(), &["derive", "--check"]);
    assert!(
        check.status.success(),
        "`fid derive --check` failed with keys set on a tree derived without them.\n\
         That is the CI-visible form of a model having reached an artifact.\n\n{}",
        text(&check)
    );
}

#[test]
fn derive_check_passes_without_keys_after_deriving_with_them() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let root = product(tmp.path(), home.path());

    assert!(run_with_keys(&root, home.path(), &["derive"])
        .status
        .success());

    // The reverse direction, which is the one a contributor hits: they have a
    // key, CI does not.
    let check = run_without_keys(&root, home.path(), &["derive", "--check"]);
    assert!(
        check.status.success(),
        "`fid derive --check` failed without keys on a tree derived with them.\n\
         A contributor with a key would produce artifacts CI cannot reproduce.\n\n{}",
        text(&check)
    );
}

#[test]
fn dash_and_doctor_report_the_same_state_with_and_without_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let root = product(tmp.path(), home.path());
    assert!(run_without_keys(&root, home.path(), &["derive"])
        .status
        .success());

    // `fid dash --json` is what agents read to learn product state, and
    // `fid doctor` is what gates a contributor before they start. Either one
    // answering differently because a key is present would make an agent's
    // view of the repository depend on the operator's shell.
    for args in [vec!["dash", "--json"], vec!["doctor"]] {
        let a = run_without_keys(&root, home.path(), &args);
        let b = run_with_keys(&root, home.path(), &args);
        assert_eq!(
            String::from_utf8_lossy(&a.stdout),
            String::from_utf8_lossy(&b.stdout),
            "`fid {}` reports differently when an AI key is present",
            args.join(" ")
        );
        assert_eq!(
            a.status.code(),
            b.status.code(),
            "`fid {}` exits differently when an AI key is present",
            args.join(" ")
        );
    }
}
