"""The paper's injected-mistake test: 20 markers and 5 controls.

Each marker is one realistic mistake a person or an agent makes while
building a product: a fact changed in one place and not another, a derived
file edited by hand, a value typed where a declaration exists. The runner
applies it to a clean copy of a product, runs the gates in the order CI
runs them, records which gates fail and how long each took, and resets.

A marker passes when at least one gate fails on it. A control passes when
every gate passes. The test passes when all 25 do.

    python3 docs/paper/markers/run.py [--fid PATH] [--only ID,ID] [--out DIR]

Products:
  web    a fresh `fid new --locales en,sr` with ten software capabilities,
         the same product `tests/fresh_product.rs` derives clean
  stick  examples/sensor-stick, in a git worktree of this commit, so its
         firmware and page build against this repository's crates

Needs: cargo (with the thumbv6m-none-eabi target), node, python3 with
ortools (placement), and network once for `npx typescript`.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

REPO = Path(__file__).resolve().parents[3]

# ── editing helpers ──────────────────────────────────────────────────────────


def edit(path: Path, old: str, new: str, count: int = 1) -> None:
    text = path.read_text()
    if old not in text:
        raise SystemExit(f"marker setup: {old!r} not found in {path}")
    path.write_text(text.replace(old, new, count))


def edit_re(path: Path, pattern: str, new: str) -> None:
    text = path.read_text()
    out, n = re.subn(pattern, new, text, count=1, flags=re.M)
    if n == 0:
        raise SystemExit(f"marker setup: /{pattern}/ not found in {path}")
    path.write_text(out)


def json_set(path: Path, dotted: str, value: str) -> None:
    data = json.loads(path.read_text())
    *parents, leaf = dotted.split(".")
    node = data
    for key in parents:
        node = node.setdefault(key, {})
    node[leaf] = value
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


# ── the markers ──────────────────────────────────────────────────────────────


@dataclass
class Case:
    id: str
    product: str  # "web" | "stick"
    area: str
    kind: str  # the class of drift it tests
    mistake: str
    apply: Callable[[Path], None]
    rederive: bool  # the person ran `fid derive` after the change
    control: bool = False


W = "web"
S = "stick"
STICK = "examples/sensor-stick"


def st(root: Path, rel: str) -> Path:
    return root / STICK / rel


# The classes of drift, from the paper's Table II.
STALE = "stale output"
HAND = "derived file edited"
INCOMPLETE = "incomplete declaration"
RULE = "declaration breaks a rule"
CONFLICT = "declarations conflict"
CONSUMER = "consumer out of date"
BYPASS = "declaration bypassed"
MEANING = "copied, not translated"

CASES: list[Case] = [
    # ── software: the web product ──
    Case("W1", W, "i18n", INCOMPLETE, "UI string added in English only",
         lambda r: json_set(r / "messages/en.json", "action.delete", "Delete"), True),
    Case("W2", W, "i18n", STALE, "English string edited, not re-derived",
         lambda r: json_set(r / "messages/en.json", "action.save", "Save"), False),
    Case("W3", W, "i18n", HAND, "Derived message module edited by hand",
         lambda r: edit(r / "apps/web/src/generated/messages.ts", "Save changes", "Save all"), False),
    Case("W4", W, "content", INCOMPLETE, "Post published in English only",
         lambda r: shutil.copy(r / "content/posts/en/hello.md", r / "content/posts/en/second.md"), True),
    Case("W5", W, "SEO", INCOMPLETE, "Page declared without title or description",
         lambda r: edit(r / "fiducial.toml", 'pages = ["/"]', 'pages = ["/", "/about"]'), True),
    Case("W6", W, "brand", STALE, "Domain changed, not re-derived",
         lambda r: edit(r / "fiducial.toml", 'domain = "example.com"', 'domain = "example.org"'), False),
    Case("W7", W, "deploy", HAND, "Derived Worker config edited by hand",
         lambda r: edit(r / "apps/worker/wrangler.toml", 'compatibility_date = "2025-01-01"',
                        'compatibility_date = "2025-06-01"'), False),
    Case("W8", W, "design", RULE, "Secondary text below 4.5 : 1 contrast",
         lambda r: edit(r / "design-system.md", 'muted-foreground     = "oklch(0.47 0 0)"',
                        'muted-foreground     = "oklch(0.75 0 0)"'), True),
    Case("W9", W, "database", HAND, "Applied migration edited in place",
         lambda r: edit_re(r / "migrations/0001_grants.sql", r"^CREATE TABLE", "CREATE TABLE IF NOT EXISTS"), False),
    Case("W10", W, "i18n", MEANING, "Serbian string left in English",
         lambda r: json_set(r / "messages/sr.json", "action.cancel", "Cancel"), True),
    # ── hardware, firmware and their interface: the sensor stick ──
    Case("H1", S, "hardware", STALE, "Sensor pin moved, not re-derived",
         lambda r: edit(st(r, "hardware/product.toml"), 'GPIO4 = "SENSOR_SDA"', 'GPIO8 = "SENSOR_SDA"'), False),
    Case("H2", S, "firmware", HAND, "Derived pin map edited by hand",
         lambda r: edit(st(r, "hardware/generated/board.rs"), "$p.PIN_4", "$p.PIN_8"), False),
    Case("H3", S, "circuit", RULE, "USB-C pull-down 5.1 kΩ → 22 kΩ",
         lambda r: edit_re(st(r, "hardware/product.toml"),
                           r'(id\s+= "r-cc1"[\s\S]*?value\s+= )"5\.1k"', r'\1"22k"'), True),
    Case("H4", S, "placement", CONFLICT, "Sensor zone contradicts its vent",
         lambda r: edit_re(st(r, "hardware/product.toml"),
                           r'(id\s+= "sensor"[\s\S]*?zone\s+= )"sensor"', r'\1"usb"'), True),
    Case("H5", S, "cost", RULE, "Cost ceiling lowered to USD 3",
         lambda r: edit_re(st(r, "hardware/product.toml"), r"^ceiling\s+= 15\.0", "ceiling  = 3.0"), True),
    Case("H6", S, "interface", CONSUMER, "Net renamed, code not updated",
         lambda r: st(r, "hardware/product.toml").write_text(
             st(r, "hardware/product.toml").read_text().replace("SENSOR_SDA", "SENSOR_DATA")), True),
    Case("H7", S, "firmware", BYPASS, "Pin named directly, then moved",
         lambda r: (edit(st(r, "firmware/rp2040/src/main.rs"), "board::sensor_sda!(p)", "p.PIN_4"),
                    edit(st(r, "hardware/product.toml"), 'GPIO4 = "SENSOR_SDA"', 'GPIO8 = "SENSOR_SDA"')), True),
    Case("H8", S, "protocol", HAND, "Message layout changed in firmware only",
         lambda r: edit(st(r, "protocol/messages.rs"),
                        "b[1..3].copy_from_slice(&self.centi_celsius.to_le_bytes());\n"
                        "            b[3..5].copy_from_slice(&self.centi_percent_rh.to_le_bytes());",
                        "b[1..3].copy_from_slice(&self.centi_percent_rh.to_le_bytes());\n"
                        "            b[3..5].copy_from_slice(&self.centi_celsius.to_le_bytes());"), False),
    Case("H9", S, "firmware", BYPASS, "Sensor I²C address typed (and mistyped)",
         lambda r: edit(st(r, "firmware/rp2040/src/main.rs"),
                        "i2c.write_async(board::SENSOR_I2C_ADDRESS, board::SENSOR_INIT)",
                        "i2c.write_async(0x39, board::SENSOR_INIT)"), True),
    Case("H10", S, "mechanical", CONFLICT, "Case narrowed 36 → 28 mm",
         lambda r: edit_re(st(r, "hardware/product.toml"), r"^width_mm = 36\.0", "width_mm = 28.0"), True),
    # ── controls: correct changes and no change ──
    Case("C1", S, "hardware", "correct change", "Sensor pin moved to GPIO8, re-derived",
         lambda r: edit(st(r, "hardware/product.toml"), 'GPIO4 = "SENSOR_SDA"', 'GPIO8 = "SENSOR_SDA"'),
         True, control=True),
    Case("C2", W, "i18n", "correct change", "UI string added in both languages",
         lambda r: (json_set(r / "messages/en.json", "action.delete", "Delete"),
                    json_set(r / "messages/sr.json", "action.delete", "Obriši")), True, control=True),
    Case("C3", S, "protocol", "correct change", "Comfortable band widened to 65 %RH",
         lambda r: edit_re(st(r, "protocol.toml"), r"^max\s+= 60\.0", "max   = 65.0"), True, control=True),
    Case("C4", W, "-", "no change", "Web product re-derived unchanged",
         lambda r: None, True, control=True),
    Case("C5", S, "-", "no change", "Sensor stick re-derived unchanged",
         lambda r: None, True, control=True),
]

# ── gates ────────────────────────────────────────────────────────────────────


@dataclass
class Gate:
    name: str
    cmd: list[str]
    cwd: str  # relative to the product root


@dataclass
class Result:
    id: str
    gates: list[dict] = field(default_factory=list)

    @property
    def failed(self) -> list[str]:
        return [g["gate"] for g in self.gates if not g["ok"]]


def gates_for(product: str, fid: str, rederive: bool) -> list[Gate]:
    g = []
    if rederive:
        g.append(Gate("derive", [fid, "derive"], "."))
    g.append(Gate("--check", [fid, "derive", "--check"], "."))
    if product == S:
        g += [
            Gate("firmware build", ["cargo", "build", "--release", "--quiet"], "firmware/rp2040"),
            Gate("firmware tests", ["cargo", "test", "--quiet", "-p", "firmware-shared",
                                    "--target", "x86_64-unknown-linux-gnu"], "firmware"),
            Gate("web test", ["node", "--experimental-strip-types", "--test",
                              "web/src/reading.test.ts"], "."),
            Gate("type check", ["npx", "-y", "-p", "typescript@5.9", "tsc", "-p", "web"], "."),
        ]
    return g


def run_gate(gate: Gate, root: Path, env: dict) -> dict:
    t0 = time.monotonic()
    p = subprocess.run(gate.cmd, cwd=root / gate.cwd, env=env,
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    dt = time.monotonic() - t0
    out = p.stdout
    # Keep the error, not the progress lines around it.
    lines = [l for l in out.splitlines() if l.strip()]
    err = [l for l in lines if re.search(r"error|✗|fail|refus|missing|not |cannot|outside|stale", l, re.I)]
    return {"gate": gate.name, "ok": p.returncode == 0, "seconds": round(dt, 2),
            "message": "\n".join((err or lines)[:12]),
            "output": "\n".join(lines[-40:]) if p.returncode else ""}


# ── products ─────────────────────────────────────────────────────────────────


def git(root: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=root, check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def make_web(base: Path, fid: str) -> Path:
    subprocess.run([fid, "new", "--locales", "en,sr", "w"], cwd=base, check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    root = base / "w"
    for cap in ["i18n", "content", "brand", "seo", "legal", "design",
                "adapters", "deploy", "migrations", "identity"]:
        subprocess.run([fid, "add", "capability", cap], cwd=root, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    subprocess.run([fid, "derive"], cwd=root, check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    git(root, "add", "-A")
    git(root, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "base")
    return root


def make_stick(base: Path) -> Path:
    wt = base / "repo"
    subprocess.run(["git", "worktree", "add", "--detach", str(wt), "HEAD"], cwd=REPO, check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return wt


def reset(git_root: Path, sub: str = ".") -> None:
    git(git_root, "checkout", "--", sub)
    git(git_root, "clean", "-fdq", "--", sub)


# ── main ─────────────────────────────────────────────────────────────────────


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--fid", default=str(REPO / "target/debug/fid"))
    ap.add_argument("--only", default="")
    ap.add_argument("--out", default=str(Path(__file__).parent))
    a = ap.parse_args()
    fid = str(Path(a.fid).resolve())
    only = set(filter(None, a.only.split(",")))
    cases = [c for c in CASES if not only or c.id in only]

    env = dict(os.environ)
    # One target directory for every firmware build, outside the worktree, so
    # a reset never throws the build away and each marker rebuilds one crate.
    env.setdefault("CARGO_TARGET_DIR", str(REPO / "target/markers"))
    env["RUST_BACKTRACE"] = "0"

    version = subprocess.run([fid, "--version"], capture_output=True, text=True).stdout.strip()
    commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=REPO,
                            capture_output=True, text=True).stdout.strip()

    base = Path(tempfile.mkdtemp(prefix="fid-markers-"))
    results = []
    try:
        web = make_web(base, fid) if any(c.product == W for c in cases) else None
        wt = make_stick(base) if any(c.product == S for c in cases) else None
        for c in cases:
            if c.product == W:
                root, git_root, sub = web, web, "."
            else:
                root, git_root, sub = wt / STICK, wt, STICK
            c.apply(git_root)
            res = Result(c.id)
            for gate in gates_for(c.product, fid, c.rederive):
                res.gates.append(run_gate(gate, root, env))
            # A control must also leave every committed file as it was when
            # nothing changed: derive on an unchanged product rewrites nothing.
            if c.mistake.endswith("unchanged"):
                diff = subprocess.run(["git", "status", "--porcelain", "--", sub], cwd=git_root,
                                      capture_output=True, text=True).stdout.strip()
                res.gates.append({"gate": "no diff", "ok": diff == "", "seconds": 0, "message": diff})
            # What the change rewrote: for a correct change, how far one edit
            # reaches (the lock itself aside).
            changed = subprocess.run(["git", "status", "--porcelain", "--", sub], cwd=git_root,
                                     capture_output=True, text=True).stdout.splitlines()
            changed = [l[3:] for l in changed if not l.endswith("fiducial.lock")]
            caught = bool(res.failed)
            ok = (not caught) if c.control else caught
            row = {"id": c.id, "product": c.product, "area": c.area, "kind": c.kind,
                   "mistake": c.mistake, "control": c.control, "rederive": c.rederive,
                   "caught_by": res.failed, "pass": ok, "changed": changed, "gates": res.gates}
            results.append(row)
            first = res.failed[0] if res.failed else "—"
            print(f"{'PASS' if ok else 'FAIL'}  {c.id:<4} {c.mistake:<46} {first}", flush=True)
            reset(git_root, sub)
    finally:
        if (base / "repo").exists():
            subprocess.run(["git", "worktree", "remove", "--force", str(base / "repo")], cwd=REPO)
        shutil.rmtree(base, ignore_errors=True)

    out = Path(a.out)
    markers = [r for r in results if not r["control"]]
    controls = [r for r in results if r["control"]]
    summary = {
        "fid": version, "commit": commit,
        "markers_caught": sum(r["pass"] for r in markers), "markers": len(markers),
        "controls_passed": sum(r["pass"] for r in controls), "controls": len(controls),
    }
    if not only:
        (out / "results.json").write_text(json.dumps({"summary": summary, "results": results},
                                                     ensure_ascii=False, indent=2) + "\n")
        lines = [f"# Injected mistakes — {version} ({commit})", "",
                 f"Markers caught: **{summary['markers_caught']} / {summary['markers']}**. "
                 f"Controls passed: **{summary['controls_passed']} / {summary['controls']}**.", "",
                 "| ID | Area | Class | Mistake | Caught by |", "|---|---|---|---|---|"]
        for r in results:
            by = ", ".join(r["caught_by"]) or ("passes every gate" if r["control"] else "**not caught**")
            lines.append(f"| {r['id']} | {r['area']} | {r['kind']} | {r['mistake']} | {by} |")
        (out / "results.md").write_text("\n".join(lines) + "\n")
    print(json.dumps(summary))
    return 0 if all(r["pass"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
