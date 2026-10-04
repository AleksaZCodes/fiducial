//! Placement by an engine, behind a contract (spec 2026-10-01).
//!
//! `fid-hardware` states where parts may go as a **placement model** — items,
//! rotations, pads, regions, and constraints of a few generic kinds — and the
//! capability's `hardware/place.py` solves it with OR-Tools CP-SAT. This file
//! is the seam: it runs the solver and caches its answer.
//!
//! The answer is a derived artifact like any other: `fid derive` writes the
//! model to `hardware/generated/placement-model.json` and the answer, stamped
//! with the model's hash, to `hardware/generated/placement.json`. When the
//! committed answer's stamp matches the model, it is reused and no solver
//! runs — so `fid derive --check` on an unchanged product needs no Python.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// The solver, embedded: the same file `fid add` installs as hardware/place.py.
const PLACE_PY: &str = include_str!("../capabilities/hardware/hardware/place.py");
const ANSWER: &str = "hardware/generated/placement.json";

/// Where each item went: its centre and its turn.
pub type Answer = BTreeMap<String, ((f64, f64), f64)>;

pub struct Placed {
    pub model: String,
    pub answer: String,
    pub items: Answer,
}

fn sha(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Solve `model`, or reuse the committed answer to exactly this model.
pub fn solve(root: &Path, model: &Value) -> Result<Placed> {
    solve_at(root, model, ANSWER, "board parts")
}

/// The case floor's model: the sockets and the board, inside the cavity.
pub const FLOOR_ANSWER: &str = "hardware/generated/floor.json";

/// Set by `fid derive --check`: answer only from the committed answers and
/// never run the solver, so a check needs no Python and writes nothing. A
/// model whose answer is not committed is then an error the check reports.
static CHECK_ONLY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn check_only(on: bool) {
    CHECK_ONLY.store(on, std::sync::atomic::Ordering::SeqCst);
}

/// Solve a model whose answer is kept at `answer_path`; `what` names what is
/// being placed, for the messages.
pub fn solve_at(root: &Path, model: &Value, answer_path: &str, what: &str) -> Result<Placed> {
    let text = serde_json::to_string_pretty(model)? + "\n";
    let stamp = sha(&text);
    let previous = std::fs::read_to_string(root.join(answer_path))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());
    let cached = previous
        .clone()
        .filter(|v| v["model_sha256"] == stamp.as_str());
    let answer = match cached {
        Some(v) => v,
        None if CHECK_ONLY.load(std::sync::atomic::Ordering::SeqCst) => {
            anyhow::bail!(
                "the {what} model changed since {answer_path} was solved (run `fid derive`)"
            )
        }
        None => {
            // The last answer seeds the search (not the stamp: it answered
            // another model), so a small change moves little.
            let hint = previous.map(|v| v["items"].clone()).unwrap_or(Value::Null);
            let input = serde_json::to_string(&json!({ "model": model, "hint": hint }))?;
            let mut v = run(&input)?;
            v["model_sha256"] = json!(stamp);
            v
        }
    };
    let ok = matches!(
        answer["status"].as_str(),
        Some("optimal") | Some("feasible")
    );
    // The problem as the solver saw it, for a person or an agent to read.
    let dump = root.join(format!(
        "hardware/build/{}",
        Path::new(answer_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("placement.json")
            .replace(".json", "-model.json")
    ));
    if !ok {
        if let Some(dir) = dump.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&dump, &text);
    }
    match answer["status"].as_str() {
        Some("optimal") | Some("feasible") => {}
        Some("infeasible") => {
            let named: Vec<String> = answer["conflict"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str())
                        .map(|s| format!("  - {s}"))
                        .collect()
                })
                .unwrap_or_default();
            bail!(
                "the {what} cannot all be placed: these declarations cannot hold together \
                 (with every part kept clear of every other):\n{}\n\
                 relax one of them, or give them room; the model is in {}",
                if named.is_empty() {
                    "  (the solver named none)".to_string()
                } else {
                    named.join("\n")
                },
                dump.display()
            )
        }
        other => bail!(
            "the placement solver found no placement of the {what} within its effort ({}); \
             raise board.placement_effort or give them room",
            other.unwrap_or("no status")
        ),
    }
    let mut items = Answer::new();
    for (id, v) in answer["items"].as_object().into_iter().flatten() {
        let c = &v["centre_mm"];
        items.insert(
            id.clone(),
            (
                (
                    c[0].as_f64()
                        .ok_or_else(|| anyhow!("{answer_path}: `{id}` has no centre"))?,
                    c[1].as_f64()
                        .ok_or_else(|| anyhow!("{answer_path}: `{id}` has no centre"))?,
                ),
                v["rotation_deg"].as_f64().unwrap_or(0.0),
            ),
        );
    }
    Ok(Placed {
        model: text,
        answer: serde_json::to_string_pretty(&answer)? + "\n",
        items,
    })
}

/// The interpreter: `FID_PYTHON`, else `python3`.
fn python() -> String {
    std::env::var("FID_PYTHON").unwrap_or_else(|_| "python3".into())
}

fn run(input: &str) -> Result<Value> {
    let py = python();
    let mut child = Command::new(&py)
        .arg("-c")
        .arg(PLACE_PY)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!("placing the board parts needs `{py}` with ortools (pip install -r hardware/requirements.txt)")
        })?;
    child
        .stdin
        .take()
        .expect("piped")
        .write_all(input.as_bytes())?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("No module named 'ortools'") {
            bail!("placing the board parts needs ortools for `{py}`: pip install -r hardware/requirements.txt");
        }
        bail!("the placement solver failed:\n{err}");
    }
    serde_json::from_slice(&out.stdout).context("reading the placement solver's answer")
}
