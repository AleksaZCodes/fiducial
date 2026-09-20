/**
 * Tests for the advisory layer.
 *
 * Everything here is the *deterministic* half — the gathering and the weighing.
 * That half is the larger one on purpose, and testing it is what keeps this
 * package honest: if the code that narrows candidates and ranks answers is
 * correct, the model is left with a small, well-posed question, and a bad answer
 * costs a sentence a human ignores. If this half is wrong, the model is
 * confidently answering about the wrong thing and the output looks just as
 * plausible.
 *
 * No test here makes a network call. The model's own behaviour is not this
 * suite's business; how we ask and how we read the reply is.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { DIFF_QUESTIONS, DIFF_WEIGHTS, duplicateOfQuestion, weigh } from "./checks.js";
import { addedLines, collectDiff, fitDiff, numericCandidates } from "./gather.js";
import { resolveKey } from "./key.js";

// ── Question shape ────────────────────────────────────────────────────────────

describe("question set", () => {
  it("only uses the three documented types", () => {
    for (const [name, q] of Object.entries(DIFF_QUESTIONS)) {
      assert.ok(["noul", "choice", "score"].includes(q.type), `${name}: ${q.type}`);
    }
  });

  it("gives every noul both criteria, phrased so true means a problem", () => {
    for (const [name, q] of Object.entries(DIFF_QUESTIONS)) {
      if (q.type !== "noul") continue;
      assert.ok(q.criteria?.true, `${name} missing true criteria`);
      assert.ok(q.criteria?.false, `${name} missing false criteria`);
    }
  });

  it("gives every choice an escape option", () => {
    // A model answers only within the options given. Without an escape, a clean
    // diff is forced to pick a violation — the failure mode this asserts away.
    for (const [name, q] of Object.entries(DIFF_QUESTIONS)) {
      if (q.type !== "choice") continue;
      const keys = Object.keys(q.criteria);
      assert.ok(
        keys.some((k) => k.includes("already") || k.includes("none")),
        `${name} has no escape option: ${keys.join(", ")}`,
      );
    }
  });

  it("stays inside the API's option and level limits", () => {
    for (const [name, q] of Object.entries(DIFF_QUESTIONS)) {
      if (q.type === "choice") {
        assert.ok(Object.keys(q.criteria).length <= 255, `${name} over 255 options`);
      }
      if (q.type === "score") {
        assert.ok(q.criteria.length >= 2 && q.criteria.length <= 10, `${name} bad level count`);
      }
    }
  });

  it("has a weight and advice for every question that can produce a finding", () => {
    for (const name of Object.keys(DIFF_QUESTIONS)) {
      assert.ok(DIFF_WEIGHTS[name], `${name} has no weight/advice entry`);
      assert.ok(DIFF_WEIGHTS[name].advice.length > 10, `${name} advice is not actionable`);
    }
  });

  it("does not ask what a deterministic check already answers", () => {
    // The guard, derive --check and the i18n pipeline own these. Asking a model
    // would trade a certain answer for a probable one.
    const blob = JSON.stringify(DIFF_QUESTIONS).toLowerCase();
    for (const forbidden of ["contrast ratio", "push to main", "missing translation"]) {
      assert.ok(!blob.includes(forbidden), `asks about \`${forbidden}\`, which code decides`);
    }
  });
});

describe("duplicateOfQuestion", () => {
  it("puts existing declarations in the options and keeps an escape", () => {
    const q = duplicateOfQuestion("pcb_height = 1.6", {
      "board.thickness": "1.6 — declared in fiducial.toml",
    });
    assert.equal(q.type, "choice");
    assert.ok("none_of_these" in q.criteria);
    assert.ok("board.thickness" in q.criteria);
  });

  it("carries the candidate in structured instructions", () => {
    const q = duplicateOfQuestion("x = 2", {});
    assert.equal(typeof q.instructions, "object");
    assert.equal(q.instructions.candidate, "x = 2");
  });
});

// ── Weighing ──────────────────────────────────────────────────────────────────

describe("weigh", () => {
  it("is silent below the mention threshold", () => {
    const f = weigh({ duplicate_declaration: { type: "noul", noul: 0.2 } });
    assert.equal(f.length, 0);
  });

  it("marks a hint between thresholds and a firm read above", () => {
    const hint = weigh({ duplicate_declaration: { type: "noul", noul: 0.6 } });
    assert.equal(hint[0].firm, false);
    const firm = weigh({ duplicate_declaration: { type: "noul", noul: 0.95 } });
    assert.equal(firm[0].firm, true);
  });

  it("ranks by probability times weight, not probability alone", () => {
    // comment_explains_what is weighted 0.3, duplicate_declaration 1.0 — so a
    // near-certain comment nit must not outrank a likely duplicate fact.
    const f = weigh({
      comment_explains_what: { type: "noul", noul: 0.99 },
      duplicate_declaration: { type: "noul", noul: 0.85 },
    });
    assert.equal(f[0].key, "duplicate_declaration");
  });

  it("treats the layer escape option as clean", () => {
    const f = weigh({
      misplaced_layer: {
        type: "choice",
        choice: "already_at_the_right_layer",
        probabilities: { already_at_the_right_layer: 0.9 },
        confidence: 0.9,
      },
    });
    assert.equal(f.length, 0);
  });

  it("reports a real layer choice with its probability", () => {
    const f = weigh({
      misplaced_layer: {
        type: "choice",
        choice: "l0_rust_core",
        probabilities: { l0_rust_core: 0.88, l3_framework_ui: 0.12 },
        confidence: 0.8,
      },
    });
    assert.equal(f.length, 1);
    assert.match(f[0].title, /l0_rust_core/);
    assert.equal(f[0].probability, 0.88);
  });

  it("drops a choice whose distribution is too flat to mean anything", () => {
    // The top option can "win" a near-uniform spread. Acting on that is acting
    // on noise, so confidence gates it independently of probability.
    const f = weigh({
      misplaced_layer: {
        type: "choice",
        choice: "l0_rust_core",
        probabilities: { l0_rust_core: 0.3, l2_headless_ts: 0.28, l3_framework_ui: 0.27 },
        confidence: 0.1,
      },
    });
    assert.equal(f.length, 0);
  });

  it("ignores an answer with no weight entry rather than inventing one", () => {
    const f = weigh({ some_future_question: { type: "noul", noul: 0.99 } });
    assert.equal(f.length, 0);
  });
});

// ── Gathering ─────────────────────────────────────────────────────────────────

const DIFF = `diff --git a/src/board.ts b/src/board.ts
--- a/src/board.ts
+++ b/src/board.ts
@@ -1,3 +1,6 @@
 export const a = 1
+const BOARD_THICKNESS = 1.6
+const MAX_RETRIES = 3
+// a comment
`;

describe("addedLines", () => {
  it("tracks file and line number of added lines only", () => {
    const added = addedLines(DIFF);
    assert.deepEqual(
      added.map((a) => a.text),
      ["const BOARD_THICKNESS = 1.6", "const MAX_RETRIES = 3", "// a comment"],
    );
    assert.equal(added[0].file, "src/board.ts");
    assert.equal(added[0].line, 2);
  });
});

describe("numericCandidates", () => {
  it("keeps a plausible physical quantity", () => {
    const c = numericCandidates(addedLines(DIFF));
    assert.ok(c.some((x) => x.name === "BOARD_THICKNESS" && x.value === 1.6));
  });

  it("drops names that are certainly not physical", () => {
    const c = numericCandidates(addedLines(DIFF));
    assert.ok(!c.some((x) => x.name === "MAX_RETRIES"));
  });

  it("drops comments", () => {
    const c = numericCandidates([{ file: "f", line: 1, text: "// thickness = 1.6" }]);
    assert.equal(c.length, 0);
  });

  it("drops a value that already carries a unit — it is already a fact", () => {
    const c = numericCandidates([
      { file: "f", line: 1, text: "thickness: Quantity<Length> = 1.6 mm" },
    ]);
    assert.equal(c.length, 0);
  });

  it("drops 0 and 1 as sentinels", () => {
    const c = numericCandidates([{ file: "f", line: 1, text: "const scale = 1" }]);
    assert.equal(c.length, 0);
  });

  it("ignores test files wholesale — a fixture is not a declaration", () => {
    // Before this filter, test fixtures asserting probability values dominated
    // the candidate list and spent model attention on guaranteed noise.
    const c = numericCandidates([
      { file: "src/x.test.js", line: 1, text: "const thickness = 1.6" },
      { file: "packages/a/src/y.spec.ts", line: 1, text: "const thickness = 1.6" },
      { file: "tests/z.js", line: 1, text: "const thickness = 1.6" },
    ]);
    assert.equal(c.length, 0);
  });

  it("drops a dimensionless value whose name says it is a probability", () => {
    const c = numericCandidates([
      { file: "src/a.ts", line: 1, text: "const confidence = 0.75" },
      { file: "src/a.ts", line: 2, text: "const noul = 0.96" },
      { file: "src/a.ts", line: 3, text: "const jitter = 0.5" },
    ]);
    assert.equal(c.length, 0);
  });

  it("still keeps a sub-1 value whose name suggests a real quantity", () => {
    // The probability filter must not swallow `clearance = 0.2` (mm).
    const c = numericCandidates([{ file: "src/a.ts", line: 1, text: "const clearance = 0.2" }]);
    assert.equal(c.length, 1);
    assert.equal(c[0].name, "clearance");
  });

  it("ignores prose files, where frontmatter looks like declarations", () => {
    // Found in the wild: a markdown story's `date = 2026` came through as a
    // candidate physical quantity. A false positive costs a call and teaches
    // the reader to distrust the check.
    const c = numericCandidates([
      { file: "press/stories/en/0001-thing.md", line: 4, text: "date = 2026" },
      { file: "docs/notes.txt", line: 1, text: "thickness = 1.6" },
    ]);
    assert.equal(c.length, 0);
  });

  it("drops date and ordering names", () => {
    const c = numericCandidates([
      { file: "src/a.ts", line: 1, text: "const year = 2026" },
      { file: "src/a.ts", line: 2, text: "const priority = 3" },
    ]);
    assert.equal(c.length, 0);
  });

  it("deduplicates and respects the cap", () => {
    const lines = Array.from({ length: 80 }, (_, i) => ({
      file: "f",
      line: i,
      text: `const value_${i} = ${i + 2}.5`,
    }));
    const c = numericCandidates(lines, 10);
    assert.equal(c.length, 10);
  });
});

describe("fitDiff", () => {
  it("passes a small diff through untouched", () => {
    const { diff, omitted } = fitDiff(DIFF);
    assert.equal(diff, DIFF);
    assert.deepEqual(omitted, []);
  });

  it("drops whole files, never half a hunk, and names what it dropped", () => {
    const big = `diff --git a/a.ts b/a.ts\n${"x".repeat(500)}\n`;
    const other = `diff --git a/b.ts b/b.ts\n${"y".repeat(500)}\n`;
    const { diff, omitted } = fitDiff(big + other, 600);
    assert.ok(diff.includes("a/a.ts"));
    assert.deepEqual(omitted, ["b.ts"]);
    // Whole-file boundaries only: a truncated hunk would invite a confident
    // answer about code the model cannot see.
    assert.ok(!diff.includes("b.ts"));
  });
});

// ── Collecting, without side effects ──────────────────────────────────────────

describe("collectDiff", () => {
  /** A throwaway repo with one modified tracked file and one untracked file. */
  function repo() {
    const dir = mkdtempSync(join(tmpdir(), "advisor-git-"));
    const run = (args) => execFileSync("git", args, { cwd: dir, stdio: "ignore" });
    run(["init", "-q", "."]);
    run(["config", "user.email", "t@example.com"]);
    run(["config", "user.name", "t"]);
    writeFileSync(join(dir, "a.txt"), "base\n");
    run(["add", "a.txt"]);
    run(["commit", "-qm", "init"]);
    writeFileSync(join(dir, "a.txt"), "base\nmodified\n");
    writeFileSync(join(dir, "new.ts"), "const board_thickness = 1.6\n");
    return dir;
  }

  const status = (dir) =>
    execFileSync("git", ["status", "--porcelain"], { cwd: dir, encoding: "utf8" });

  it("leaves the git index untouched", () => {
    // The obvious implementation runs `git add -N` so untracked files show up in
    // `git diff`. That writes to the index, changing what `git status` reports
    // and what a bare `git commit` would include. This command promises to
    // change nothing, so that shortcut is banned and this test is the ban.
    const dir = repo();
    const before = status(dir);
    collectDiff(dir);
    assert.equal(status(dir), before);
  });

  it("still reviews untracked files, which is where new violations live", () => {
    const dir = repo();
    const { diff, files } = collectDiff(dir);
    assert.ok(files.includes("new.ts"));
    assert.match(diff, /\+const board_thickness = 1\.6/);
  });

  it("includes modifications to tracked files", () => {
    const dir = repo();
    const { diff, files } = collectDiff(dir);
    assert.ok(files.includes("a.txt"));
    assert.match(diff, /\+modified/);
  });

  it("reports an empty diff for a clean tree rather than throwing", () => {
    const dir = mkdtempSync(join(tmpdir(), "advisor-clean-"));
    execFileSync("git", ["init", "-q", "."], { cwd: dir, stdio: "ignore" });
    const { diff } = collectDiff(dir);
    assert.equal(diff.trim(), "");
  });

  it("returns empty rather than throwing outside a git repository", () => {
    // A hook must not crash because someone ran it somewhere unexpected.
    const dir = mkdtempSync(join(tmpdir(), "advisor-nogit-"));
    const { diff, files } = collectDiff(dir);
    assert.equal(diff.trim(), "");
    assert.deepEqual(files, []);
  });
});

// ── Key resolution ────────────────────────────────────────────────────────────

describe("resolveKey", () => {
  it("prefers the gateway key and reports its source", () => {
    const r = resolveKey({ OPENROUTER_API_KEY: "k", TYPESAFE_API_KEY: "t" });
    assert.equal(r.vendor, "openrouter");
    assert.equal(r.env.OPENROUTER_API_KEY, "k");
    assert.match(r.source, /environment/);
  });

  it("falls back to the direct key", () => {
    const r = resolveKey({ TYPESAFE_API_KEY: "t" });
    assert.equal(r.vendor, "typesafe");
  });

  it("reports a missing key as a normal state with the fix", () => {
    // Absence must be reportable rather than thrown: a hook with no key has to
    // exit 0 quietly, not crash.
    const r = resolveKey({ XDG_CONFIG_HOME: "/nonexistent-fiducial-test" });
    assert.equal(r.vendor, null);
    assert.match(r.reason, /fid advise key set/);
  });
});
