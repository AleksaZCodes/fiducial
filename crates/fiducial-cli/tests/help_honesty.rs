//! `fid --help` must describe what the CLI does, not when something is planned.
//!
//! Help text had drifted badly: `fid derive` — the central command of the whole
//! system — told users it was "Not yet implemented (Phase 4)" and to run
//! `cargo build` directly. `fid add app svelte` and `fid add firmware stm32`
//! were labelled "(Phase 3b)" long after they worked, while the targets that
//! genuinely do not work promised a phase that had already shipped.
//!
//! The cause is the one the platform exists to remove: a schedule is a fact
//! `PHASES.md` owns, and help text held a second copy of it. So the rule this
//! test enforces is that help text names no phase at all.

use std::{path::Path, process::Command};

fn fid_bin() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_fid"))
}

fn help(args: &[&str]) -> String {
    let out = Command::new(fid_bin())
        .args(args)
        .arg("--help")
        .output()
        .expect("failed to invoke fid");
    assert!(out.status.success(), "`fid {args:?} --help` failed");
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// Every subcommand that has its own help page.
const SUBCOMMANDS: &[&[&str]] = &[
    &[],
    &["new"],
    &["add"],
    &["add", "app"],
    &["add", "module"],
    &["add", "firmware"],
    &["capability"],
    &["derive"],
    &["graph"],
    &["dash"],
    &["doctor"],
    &["upgrade"],
];

#[test]
fn no_help_page_names_a_phase() {
    // A phase number in help text is a second declaration of the schedule
    // PHASES.md owns, and it drifts silently — every one of them was wrong by
    // the time it was found.
    let phase = regex_lite_phase();
    for args in SUBCOMMANDS {
        let text = help(args);
        assert!(
            !phase(&text),
            "`fid {} --help` names a phase; describe present capability instead:\n{text}",
            args.join(" ")
        );
    }
}

/// Crude `Phase <n>` detector — no regex dependency for one pattern.
fn regex_lite_phase() -> impl Fn(&str) -> bool {
    |text: &str| {
        text.match_indices("Phase ").any(|(i, _)| {
            text[i + 6..]
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        })
    }
}

#[test]
fn no_help_page_claims_something_is_unimplemented_when_it_works() {
    // `fid derive`, `fid graph`, and `fid dash` all work. Any of them saying
    // otherwise sends a reader to do the work by hand.
    for args in [
        vec!["derive"],
        vec!["graph"],
        vec!["dash"],
        vec!["doctor"],
        vec!["upgrade"],
    ] {
        let text = help(&args).to_lowercase();
        assert!(
            !text.contains("not yet implemented"),
            "`fid {} --help` claims to be unimplemented",
            args.join(" ")
        );
    }
}

#[test]
fn targets_that_work_are_not_marked_unavailable() {
    // svelte and stm32 install cleanly; they were labelled as pending.
    let app = help(&["add", "app"]);
    for line in app
        .lines()
        .filter(|l| l.contains("svelte") || l.contains("next"))
    {
        assert!(
            !line.to_lowercase().contains("not available"),
            "working target marked unavailable: {line}"
        );
    }
    let fw = help(&["add", "firmware"]);
    for line in fw
        .lines()
        .filter(|l| l.contains("stm32") || l.contains("rp2040"))
    {
        assert!(
            !line.to_lowercase().contains("not available"),
            "working target marked unavailable: {line}"
        );
    }
}

#[test]
fn targets_that_do_not_work_say_so() {
    // The other half of the rule: silence about a missing target is worse than
    // a stale promise, because the failure then arrives at install time.
    let app = help(&["add", "app"]);
    let mobile = app
        .lines()
        .find(|l| l.trim_start().starts_with("mobile"))
        .expect("mobile listed");
    assert!(
        mobile.to_lowercase().contains("not available"),
        "mobile does not install, and help should say so: {mobile}"
    );

    let fw = help(&["add", "firmware"]);
    let nrf = fw
        .lines()
        .find(|l| l.trim_start().starts_with("nrf52"))
        .expect("nrf52 listed");
    assert!(
        nrf.to_lowercase().contains("not available"),
        "nrf52 does not install, and help should say so: {nrf}"
    );
}
