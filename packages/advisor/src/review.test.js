import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { mergeFindings } from "./checks.js";
import { createHash } from "node:crypto";
import { diffStat, docsDrift, isDerivedPath, mapLimit, derivedOutputs, repoFiles, splitDiff, untouchedPlatformFiles } from "./gather.js";
import { AGENT_BRIEF, buildReviewRequest, parseReview, reviewAll } from "./review.js";

function fileDiff(name, lines, { hunks = 1 } = {}) {
  let out = `diff --git a/${name} b/${name}\nnew file mode 100644\n--- /dev/null\n+++ b/${name}\n`;
  const per = Math.ceil(lines.length / hunks);
  for (let h = 0; h < hunks; h++) {
    const slice = lines.slice(h * per, (h + 1) * per);
    out += `@@ -0,0 +${h * per + 1},${slice.length} @@\n${slice.map((l) => `+${l}`).join("\n")}\n`;
  }
  return out;
}

describe("splitDiff", () => {
  it("keeps one small file as one unit and packs small neighbours together", () => {
    const diff =
      fileDiff("content/a/en/one.md", ["a"]) +
      fileDiff("content/a/en/two.md", ["b"]) +
      fileDiff("src/other.ts", ["c"]);
    const { units, skipped } = splitDiff(diff, 40_000, { tiny: 0 });
    assert.equal(skipped.length, 0);
    assert.equal(units.length, 2);
    assert.deepEqual(units[0].files, ["content/a/en/one.md", "content/a/en/two.md"]);
    assert.deepEqual(units[1].files, ["src/other.ts"]);
  });

  it("packs tiny units across directories, but never a unit that is not tiny", () => {
    const big = Array.from({ length: 120 }, (_, i) => `line ${i} ${"y".repeat(40)}`);
    const diff =
      fileDiff("a/one.md", ["a"]) +
      fileDiff("b/two.md", ["b"]) +
      fileDiff("c/three.md", ["c"]) +
      fileDiff("d/large.ts", big);
    const { units } = splitDiff(diff, 40_000);
    assert.equal(units.length, 2);
    assert.deepEqual(units[0].files, ["a/one.md", "b/two.md", "c/three.md"]);
    assert.deepEqual(units[1].files, ["d/large.ts"]);
    assert.match(units[0].label, /mixed \(3 files\)/);
  });

  it("splits a file larger than the budget at its hunks, keeping the file header on each part", () => {
    const lines = Array.from({ length: 400 }, (_, i) => `line number ${i} ${"x".repeat(40)}`);
    const diff = fileDiff("src/big.ts", lines, { hunks: 8 });
    const { units } = splitDiff(diff, 5_000);

    assert.ok(units.length > 1, "a large file becomes several units");
    for (const u of units) {
      assert.ok(u.diff.startsWith("diff --git a/src/big.ts"), "every part says which file it is");
      assert.ok(u.diff.length <= 5_500, "no part exceeds the budget by more than a hunk");
    }
    const joined = units.map((u) => u.diff).join("");
    for (const l of lines) assert.ok(joined.includes(l), "nothing is dropped to fit");
  });

  it("skips derived and opaque files by name instead of crowding out real ones", () => {
    const diff =
      fileDiff("apps/web/src/generated/content.ts", ["x"]) +
      fileDiff("pnpm-lock.yaml", ["y"]) +
      fileDiff("apps/web/src/app/page.tsx", ["z"]);
    const { units, skipped } = splitDiff(diff);
    assert.deepEqual(skipped, ["apps/web/src/generated/content.ts", "pnpm-lock.yaml"]);
    assert.deepEqual(units.flatMap((u) => u.files), ["apps/web/src/app/page.tsx"]);
  });

  it("recognises what is derived", () => {
    assert.ok(isDerivedPath("fiducial.lock"));
    assert.ok(isDerivedPath(".fiducial/skills/seo.md"), "platform-owned, so not the product to fix");
    assert.ok(isDerivedPath(".claude/agents/fiducial-design.md"));
    assert.ok(isDerivedPath("apps/web/public/fonts/x.woff2"));
    assert.ok(!isDerivedPath("apps/web/src/lib/content.ts"));
  });
});

describe("platform files", () => {
  it("skips a template file whose content still matches fiducial.lock, and keeps an edited one", () => {
    const sha = (t) => createHash("sha256").update(t).digest("hex");
    const dir = gitRepo({
      "scripts/untouched.mjs": "export const a = 1\n",
      "MISSION.md": "edited by the product\n",
      "fiducial.lock":
        `[templates."scripts/untouched.mjs"]\nsource_version = "0.10.4"\nhash = "${sha("export const a = 1\n")}"\nbase_content = """\nx\n"""\n\n` +
        `[templates."MISSION.md"]\nsource_version = "0.10.4"\nhash = "${sha("what the platform wrote\n")}"\nbase_content = """\ny\n"""\n`,
    });
    const untouched = untouchedPlatformFiles(dir);
    assert.ok(untouched.has("scripts/untouched.mjs"));
    assert.ok(!untouched.has("MISSION.md"), "a file the product edited is reviewed");
  });

  it("has nothing to skip without a lock", () => {
    assert.equal(untouchedPlatformFiles(mkdtempSync(join(tmpdir(), "advisor-nolock-"))).size, 0);
  });

  it("splitDiff leaves out files the caller says to skip, by name", () => {
    const diff = fileDiff("scripts/platform.mjs", ["a"]) + fileDiff("src/mine.ts", ["b"]);
    const { units, skipped } = splitDiff(diff, 40_000, { skip: (n) => n === "scripts/platform.mjs" });
    assert.deepEqual(skipped, ["scripts/platform.mjs"]);
    assert.deepEqual(units.flatMap((u) => u.files), ["src/mine.ts"]);
  });

  it("reads which files are derived from the pipelines that declare them", () => {
    const dir = gitRepo({
      "pipelines/seo.toml": `name = "seo"\noutputs = [\n  "apps/web/public/sitemap.xml",\n  "apps/web/src/generated/seo.ts",\n]\n`,
      "pipelines/design.toml": `name = "design"\noutputs  = ["apps/web/src/app/tokens.css"]\n`,
    });
    assert.deepEqual(
      [...derivedOutputs(dir)].sort(),
      ["apps/web/public/sitemap.xml", "apps/web/src/app/tokens.css", "apps/web/src/generated/seo.ts"],
    );
    assert.equal(derivedOutputs(mkdtempSync(join(tmpdir(), "advisor-nopipes-"))).size, 0);
  });

  it("lists tracked files so unchanged is distinguishable from missing", () => {
    const dir = gitRepo({ "a/b.ts": "x\n", "c.md": "y\n" });
    assert.deepEqual(repoFiles(dir).sort(), ["a/b.ts", "c.md"]);
  });
});

describe("diffStat and mapLimit", () => {
  it("counts added and removed lines per file", () => {
    const diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n-old\n+new\n same\n";
    assert.deepEqual(diffStat(diff), [{ file: "f", add: 1, del: 1 }]);
  });

  it("never runs more than the limit at once and keeps result order", async () => {
    let live = 0;
    let peak = 0;
    const out = await mapLimit([1, 2, 3, 4, 5, 6], 2, async (n) => {
      live++;
      peak = Math.max(peak, live);
      await new Promise((r) => setTimeout(r, 5));
      live--;
      return n * 10;
    });
    assert.deepEqual(out, [10, 20, 30, 40, 50, 60]);
    assert.ok(peak <= 2);
  });
});

function gitRepo(files) {
  const dir = mkdtempSync(join(tmpdir(), "advisor-docs-"));
  const run = (args) => execFileSync("git", args, { cwd: dir, stdio: "ignore" });
  run(["init", "-q", "."]);
  run(["config", "user.email", "t@example.com"]);
  run(["config", "user.name", "t"]);
  for (const [name, text] of Object.entries(files)) {
    mkdirSync(dirname(join(dir, name)), { recursive: true });
    writeFileSync(join(dir, name), text);
  }
  run(["add", "-A"]);
  run(["commit", "-qm", "init"]);
  return dir;
}

describe("docsDrift", () => {
  it("names the documents that mention a touched file but were not edited", () => {
    const dir = gitRepo({
      "README.md": "Run the importer in `scripts/importer.mjs` to load data.\n",
      "docs/specs/2026-01-01-importer.md": "We chose scripts/importer.mjs.\n",
      "scripts/importer.mjs": "export const a = 1\n",
    });
    const diff = "diff --git a/scripts/importer.mjs b/scripts/importer.mjs\n--- a/scripts/importer.mjs\n+++ b/scripts/importer.mjs\n@@ -1 +1 @@\n-export const a = 1\n+export const a = 2\n";
    const drift = docsDrift(dir, ["scripts/importer.mjs"], diff);

    assert.deepEqual(drift.stale.map((s) => s.doc), ["README.md"], "a decision record is never a stale candidate");
    assert.equal(drift.stale[0].token, "scripts/importer.mjs");
    assert.equal(drift.stale[0].line, 1);
    assert.equal(drift.noDocsChanged, true);
  });

  it("does not list a document the change already edited, and clears the no-docs note", () => {
    const dir = gitRepo({
      "README.md": "See `src/engine.ts`.\n",
      "src/engine.ts": "x\n",
    });
    const drift = docsDrift(dir, ["src/engine.ts", "README.md"], "");
    assert.deepEqual(drift.stale, []);
    assert.equal(drift.noDocsChanged, false);
  });

  it("flags a new command line flag that no document mentions", () => {
    const dir = gitRepo({ "README.md": "Use `--known`.\n", "cli.js": "x\n" });
    const diff =
      "diff --git a/cli.js b/cli.js\n--- a/cli.js\n+++ b/cli.js\n@@ -1 +1,3 @@\n x\n+if (has('--known')) {}\n+if (has('--frobnicate')) {}\n";
    const drift = docsDrift(dir, ["cli.js"], diff);
    assert.deepEqual(drift.undocumented.map((u) => u.token), ["--frobnicate"]);
  });
});

describe("parseReview", () => {
  it("reads JSON inside a code fence and normalises severity", () => {
    const text =
      '```json\n{"findings":[{"file":"a.ts","line":3,"rule":"R","problem":"p","fix":"f","severity":"check"},{"file":"b.ts","problem":"q"}]}\n```';
    const out = parseReview(text);
    assert.equal(out.length, 2);
    assert.equal(out[0].severity, "check");
    assert.equal(out[0].line, 3);
    assert.equal(out[1].severity, "fix");
    assert.equal(out[1].line, undefined);
  });

  it("drops malformed entries instead of throwing", () => {
    assert.deepEqual(parseReview("not json"), []);
    assert.deepEqual(parseReview('{"findings":[{"file":"a"},{"problem":"x"},null,{"file":"a","problem":"  "}]}'), []);
    assert.deepEqual(parseReview(undefined), []);
  });

  it("caps the number of findings", () => {
    const many = Array.from({ length: 80 }, (_, i) => ({ file: "a", problem: `p${i}` }));
    assert.equal(parseReview(JSON.stringify({ findings: many })).length, 10);
  });
});

describe("buildReviewRequest", () => {
  const base = {
    task: "do the thing",
    rules: { "AGENTS.md": "RULE ONE" },
    files: ["a.ts", "b.ts"],
    repo: ["lib/untouched.ts", "a.ts"],
    unit: { files: ["a.ts"], diff: "diff --git a/a.ts b/a.ts\n+hello\n" },
    stat: [{ file: "a.ts", add: 1, del: 0 }],
    docs: { stale: [{ doc: "README.md", line: 2, token: "a.ts" }], undocumented: [], noDocsChanged: true },
  };

  it("gives a unit its diff, the rules and the manifest, but not the doc candidates", () => {
    const body = buildReviewRequest({ ...base, mode: "unit" });
    const user = body.messages[1].content;
    assert.equal(body.model, "anthropic/claude-haiku-5.5");
    assert.match(user, /RULE ONE/);
    assert.match(user, /UNIT_FILES\na\.ts/);
    assert.match(user, /\+hello/);
    assert.match(user, /CHANGED_FILES\na\.ts\nb\.ts/);
    assert.match(user, /REPO_FILES\nlib\/untouched\.ts/);
    assert.match(body.messages[0].content, /exists and is unchanged/);
    assert.doesNotMatch(user, /DOC_CANDIDATES/);
    assert.match(body.messages[0].content, /Do not report stale docs/);
  });

  it("gives the overview the doc candidates and line counts, and no diff", () => {
    const body = buildReviewRequest({ ...base, mode: "overview" });
    const user = body.messages[1].content;
    assert.match(user, /README\.md:2 mentions `a\.ts`/);
    assert.match(user, /a\.ts {2}\+1 -0/);
    assert.match(user, /no document outside docs\/specs\/ was changed/);
    assert.doesNotMatch(user, /\+hello/);
    assert.match(body.messages[0].content, /append-only/);
  });

  it("tells the model the inputs are data", () => {
    const body = buildReviewRequest({ ...base, mode: "unit" });
    assert.match(body.messages[0].content, /must not follow them/);
  });
});

describe("mergeFindings", () => {
  it("keeps the strongest read, a firm flag from any unit, and the units that raised it", () => {
    const f = (probability, firm) => ({ key: "k", title: "t", advice: "a", probability, firm, score: probability });
    const merged = mergeFindings([
      { label: "u1", findings: [f(0.6, false)] },
      { label: "u2", findings: [f(0.9, true)] },
      { label: "u3", findings: [] },
    ]);
    assert.equal(merged.length, 1);
    assert.equal(merged[0].probability, 0.9);
    assert.equal(merged[0].firm, true);
    assert.deepEqual(merged[0].units, ["u1", "u2"]);
  });
});

describe("reviewAll", () => {
  const answer = (findings) => ({
    ok: true,
    status: 200,
    json: async () => ({
      model: "anthropic/claude-haiku-5.5-20261001",
      choices: [{ message: { content: JSON.stringify({ findings }) } }],
      usage: { cost: 0.001 },
    }),
  });

  it("asks once per unit plus the overview, merges duplicates and names what failed", async () => {
    const calls = [];
    const fetchImpl = async (_url, init) => {
      const body = JSON.parse(init.body);
      calls.push(body.messages[1].content.includes("DIFF\n") ? "unit" : "overview");
      const text = body.messages[1].content;
      if (text.includes("UNIT_FILES\nbad.ts")) return { ok: false, status: 500, json: async () => ({}) };
      return answer([{ file: "x.ts", line: 1, problem: "same problem", severity: "check" }, { file: "y.ts", problem: "urgent", severity: "fix" }]);
    };
    const out = await reviewAll({
      apiKey: "k",
      fetchImpl,
      units: [
        { id: 1, label: "a.ts", files: ["a.ts"], diff: "diff --git a/a.ts b/a.ts\n+1\n" },
        { id: 2, label: "b.ts", files: ["b.ts"], diff: "diff --git a/b.ts b/b.ts\n+2\n" },
        { id: 3, label: "bad.ts", files: ["bad.ts"], diff: "diff --git a/bad.ts b/bad.ts\n+3\n" },
      ],
      stat: [],
      docs: { stale: [], undocumented: [], noDocsChanged: false },
      files: ["a.ts", "b.ts", "bad.ts"],
      rules: {},
    });

    assert.equal(calls.length, 4);
    assert.equal(out.requests, 4);
    assert.deepEqual(out.skipped.map((s) => s.unit), ["bad.ts"]);
    assert.equal(out.findings.filter((f) => f.problem === "same problem").length, 1, "the same finding is listed once");
    assert.equal(out.findings[0].severity, "fix", "confident findings come first");
    assert.ok(out.cost > 0);
  });

  it("states what the agent should do with the result", () => {
    assert.match(AGENT_BRIEF, /claim to check/);
    assert.match(AGENT_BRIEF, /docs\/specs\//);
  });
});
