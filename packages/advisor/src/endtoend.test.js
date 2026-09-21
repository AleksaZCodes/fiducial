/**
 * End-to-end: the real CLI, a real git repo, a real HTTP server.
 *
 * # Why this exists
 *
 * Every other test in this package covers the deterministic half in isolation.
 * The stretch none of them reach is the one that only runs when a key is
 * present: build the request, send it, parse the reply, weigh the answers, print
 * a report, choose an exit code. That stretch cannot be exercised against the
 * live API without credits, and it is exactly where a mistake is invisible —
 * a mis-read answer shape produces a confident, plausible, wrong report.
 *
 * So the vendor is replaced with a local server that speaks the documented
 * response shape, via `FIDUCIAL_ADVISE_ENDPOINT`. That verifies everything
 * except whether the real vendor agrees with its own published schema — which
 * the fixtures in `packages/adapters/src/system-one.test.js` cover by being
 * copied verbatim from it.
 *
 * The CLI is spawned as a subprocess rather than imported, because the thing
 * under test includes its argument handling and its exit code.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { createServer } from "node:http";
import { chmodSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const CLI = join(dirname(fileURLToPath(import.meta.url)), "cli.js");

/** A repo with one committed file, one modification, and one new file. */
function repo() {
  const dir = mkdtempSync(join(tmpdir(), "advisor-e2e-"));
  const run = (args) => execFileSync("git", args, { cwd: dir, stdio: "ignore" });
  run(["init", "-q", "."]);
  run(["config", "user.email", "t@example.com"]);
  run(["config", "user.name", "t"]);
  writeFileSync(join(dir, "seed.txt"), "seed\n");
  run(["add", "seed.txt"]);
  run(["commit", "-qm", "init"]);
  writeFileSync(join(dir, "board.ts"), "const board_thickness = 1.6\n");
  return dir;
}

/**
 * Serve one canned decisions response, recording what was posted.
 *
 * Returns the URL plus the captured request, so a test can assert on what left
 * the process as well as on what it printed.
 */
async function stub(response, status = 200) {
  const captured = { bodies: [], count: 0, auth: null };
  const server = createServer((req, res) => {
    let raw = "";
    req.on("data", (c) => (raw += c));
    req.on("end", () => {
      captured.count++;
      captured.auth = req.headers.authorization ?? null;
      try {
        captured.bodies.push(JSON.parse(raw));
      } catch {
        captured.bodies.push({ unparseable: raw });
      }
      res.writeHead(status, { "content-type": "application/json" });
      res.end(JSON.stringify(response));
    });
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  const { port } = server.address();
  return {
    url: `http://127.0.0.1:${port}/`,
    captured,
    // `closeAllConnections` before `close`, because Node's fetch holds the
    // socket open with keep-alive and `close()` alone waits for idle
    // connections that never go away — the suite hangs instead of failing.
    close: () =>
      new Promise((r) => {
        server.closeAllConnections?.();
        server.close(r);
      }),
  };
}

/**
 * Run the CLI in `cwd` against `endpoint`.
 *
 * Asynchronous `spawn`, never `spawnSync`. The synchronous form blocks this
 * process's event loop until the child exits, which means the stub server —
 * running in this same process — can never accept the child's request. The
 * child then waits forever for a response nobody is free to send, and the suite
 * hangs rather than failing. That deadlock cost an afternoon once; it is why
 * this helper is `async`.
 */
function runCli(cwd, endpoint, args, extraEnv = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [CLI, ...args], {
      cwd,
      env: {
        ...process.env,
        OPENROUTER_API_KEY: "sk-or-v1-stub",
        TYPESAFE_API_KEY: "",
        FIDUCIAL_ADVISE_ENDPOINT: endpoint,
        ...extraEnv,
      },
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (c) => (stdout += c));
    child.stderr.on("data", (c) => (stderr += c));
    child.on("error", reject);
    child.on("close", (code) => resolve({ stdout, stderr, code }));
  });
}

/** A response where every question comes back clean. */
const CLEAN = {
  model: "typesafe/jev-1.13-20260917",
  provider: "TypeSafe",
  answers: {
    misplaced_layer: {
      type: "choice",
      choice: "already_at_the_right_layer",
      probabilities: { already_at_the_right_layer: 0.95 },
      confidence: 0.93,
    },
    duplicate_declaration: { type: "noul", noul: 0.04 },
    bare_quantity: { type: "noul", noul: 0.02 },
    abstraction_without_escape: { type: "noul", noul: 0.05 },
    comment_explains_what: { type: "noul", noul: 0.03 },
    half_finished: { type: "noul", noul: 0.01 },
  },
  usage: { input_tokens: 400, output_tokens: 40, cost: 0.0000168 },
};

/** A response with a firm duplicate finding and a firm bare quantity. */
const DIRTY = {
  ...CLEAN,
  answers: {
    ...CLEAN.answers,
    duplicate_declaration: { type: "noul", noul: 0.93 },
    bare_quantity: { type: "noul", noul: 0.88 },
    comment_explains_what: { type: "noul", noul: 0.62 },
  },
};

describe("end to end", () => {
  it("sends the documented request shape and a bearer token", async () => {
    const s = await stub(CLEAN);
    try {
      await runCli(repo(), s.url, ["diff"]);
      assert.equal(s.captured.count, 1);
      assert.equal(s.captured.auth, "Bearer sk-or-v1-stub");

      const body = s.captured.bodies[0];
      assert.equal(body.model, "typesafe/jev-latest");
      assert.ok(typeof body.state === "object");
      assert.ok(body.state.diff.includes("board_thickness"));
      assert.ok(body.session_id.startsWith("advise-diff-"));
      // Not chat completions. The whole point of the contract.
      assert.ok(!("messages" in body));
      for (const q of Object.values(body.questions)) {
        assert.ok(["noul", "choice", "score"].includes(q.type));
      }
    } finally {
      await s.close();
    }
  });

  it("reports a clean diff and exits 0", async () => {
    const s = await stub(CLEAN);
    try {
      const r = await runCli(repo(), s.url, ["diff"]);
      assert.equal(r.code, 0);
      assert.match(r.stdout, /nothing to flag/);
    } finally {
      await s.close();
    }
  });

  it("ranks findings, marks firmness, and prints evidence and cost", async () => {
    const s = await stub(DIRTY);
    try {
      const r = await runCli(repo(), s.url, ["diff"]);
      assert.equal(r.code, 0);

      // Weighted ranking: duplicate_declaration (1.0 × 0.93) must lead.
      const firstLine = r.stdout.split("\n").find((l) => l.includes("▲"));
      assert.match(firstLine, /declared twice/);

      // The 0.62 comment answer is a hint, not a firm read.
      assert.match(r.stdout, /·.*comment/);
      // Evidence comes from the mechanical pass, so it names a real line.
      assert.match(r.stdout, /board\.ts:1\s+board_thickness = 1\.6/);
      assert.match(r.stdout, /cost: \$0\.000017/);
      assert.match(r.stdout, /Advisory only/);
    } finally {
      await s.close();
    }
  });

  it("exits 0 on findings by default, non-zero only with --strict", async () => {
    // The default is the contract: advisory output must never be the reason a
    // commit or a hook fails.
    const a = await stub(DIRTY);
    try {
      assert.equal((await runCli(repo(), a.url, ["diff"])).code, 0);
    } finally {
      await a.close();
    }

    const b = await stub(DIRTY);
    try {
      assert.equal((await runCli(repo(), b.url, ["diff", "--strict"])).code, 1);
    } finally {
      await b.close();
    }
  });

  it("stays 0 under --strict when findings are only hints", async () => {
    const hintsOnly = {
      ...CLEAN,
      answers: { ...CLEAN.answers, duplicate_declaration: { type: "noul", noul: 0.6 } },
    };
    const s = await stub(hintsOnly);
    try {
      assert.equal((await runCli(repo(), s.url, ["diff", "--strict"])).code, 0);
    } finally {
      await s.close();
    }
  });

  it("emits machine-readable findings with --json", async () => {
    const s = await stub(DIRTY);
    try {
      const r = await runCli(repo(), s.url, ["diff", "--json"]);
      const parsed = JSON.parse(r.stdout);
      assert.ok(Array.isArray(parsed.findings));
      assert.equal(parsed.findings[0].key, "duplicate_declaration");
      assert.equal(parsed.findings[0].firm, true);
      assert.equal(parsed.model, "typesafe/jev-1.13-20260917");
    } finally {
      await s.close();
    }
  });

  it("survives a 500 without failing the caller", async () => {
    // A model that is down must never be why a commit cannot happen.
    const s = await stub({ error: { message: "upstream exploded" } }, 500);
    try {
      const r = await runCli(repo(), s.url, ["diff"]);
      assert.equal(r.code, 0);
      assert.match(r.stdout, /skipped/);
    } finally {
      await s.close();
    }
  });

  it("survives a 402 out-of-credits without failing the caller", async () => {
    // The state this was actually written in: a valid key with no balance.
    const s = await stub({ error: { code: 402, message: "Insufficient credits" } }, 402);
    try {
      const r = await runCli(repo(), s.url, ["diff"]);
      assert.equal(r.code, 0);
      assert.match(r.stdout, /skipped/);
      assert.match(r.stdout, /Insufficient credits/);
      // Not retried: a balance does not refill inside one backoff window.
      assert.equal(s.captured.count, 1);
    } finally {
      await s.close();
    }
  });

  it("leaves the git index untouched across a full run", async () => {
    const s = await stub(DIRTY);
    const dir = repo();
    const status = () =>
      execFileSync("git", ["status", "--porcelain"], { cwd: dir, encoding: "utf8" });
    try {
      const before = status();
      await runCli(dir, s.url, ["diff"]);
      assert.equal(status(), before);
    } finally {
      await s.close();
    }
  });

  it("sends nothing at all under --dry-run", async () => {
    const s = await stub(CLEAN);
    try {
      const r = await runCli(repo(), s.url, ["diff", "--dry-run"]);
      assert.equal(r.code, 0);
      assert.equal(s.captured.count, 0, "--dry-run must not make a request");
      assert.match(r.stdout, /nothing sent/);
    } finally {
      await s.close();
    }
  });

  it("asks one question per new declaration in facts mode", async () => {
    const s = await stub({
      model: "typesafe/jev-1.13-20260917",
      answers: {
        dup_0: {
          type: "choice",
          choice: "none_of_these",
          probabilities: { none_of_these: 0.9 },
          confidence: 0.88,
        },
      },
      usage: { input_tokens: 100, output_tokens: 10, cost: 0.0000042 },
    });
    const dir = repo();
    writeFileSync(join(dir, "extra.toml"), 'pcb_height = "1.6"\n');
    try {
      const r = await runCli(dir, s.url, ["facts"]);
      assert.equal(r.code, 0);
      const body = s.captured.bodies[0];
      assert.ok(Object.keys(body.questions).every((k) => k.startsWith("dup_")));
      for (const q of Object.values(body.questions)) {
        assert.equal(q.type, "choice");
        // The escape option must always be offered, or a genuinely new
        // declaration is forced to match something.
        assert.ok("none_of_these" in q.criteria);
      }
    } finally {
      await s.close();
    }
  });
});

// ── Thesis ────────────────────────────────────────────────────────────────────

/**
 * A product with a declared thesis, and a fake `fid` on PATH that reports it.
 *
 * `collectThesis` shells out to `fid thesis --json` on purpose — one parser for
 * the declaration, the one with the Rust tests behind it. That makes the real
 * binary a dependency of this path, and a JS suite that needs a freshly-built
 * Rust binary on PATH is a suite that fails for reasons unrelated to what it
 * tests. So the contract is stubbed the same way the vendor is: a script that
 * speaks the documented shape.
 */
function thesisRepo(payload) {
  const dir = repo();
  const bin = join(dir, ".bin");
  mkdirSync(bin, { recursive: true });
  const fake = join(bin, "fid");
  writeFileSync(
    fake,
    `#!/bin/sh\ncat <<'JSON'\n${JSON.stringify(payload, null, 2)}\nJSON\n`,
  );
  chmodSync(fake, 0o755);
  return { dir, bin };
}

const THESIS_PAYLOAD = {
  declared: true,
  current: {
    date: "2026-09-21",
    claim: "No unverified alert ever reaches a responder.",
    arc: { problem: "A network that cries wolf is worse than no network." },
    test: { falsified_by: "Responders act on machine-only alerts at the same rate." },
    evidence: { not_yet: ["No field deployment, no pilot, no live public alerting."] },
  },
  gaps: [{ field: "arc.why_now", question: "What changed?" }],
  history: [],
};

const THESIS_CLEAN = {
  model: "typesafe/jev-1.13-20260917",
  provider: "TypeSafe",
  answers: {
    not_falsifiable: { type: "noul", noul: 0.03 },
    nobody_disagrees: { type: "noul", noul: 0.09 },
    describes_instead_of_claiming: { type: "noul", noul: 0.05 },
    falsifier_does_not_falsify: { type: "noul", noul: 0.07 },
    copy_contradicts_the_claim: { type: "noul", noul: 0.04 },
    overclaims_against_evidence: { type: "noul", noul: 0.02 },
  },
  usage: { input_tokens: 300, output_tokens: 30, cost: 0.0000121 },
};

describe("end to end — thesis", () => {
  it("asks only the questions whose inputs exist, and never leaks the diff", async () => {
    const { dir, bin } = thesisRepo(THESIS_PAYLOAD);
    writeFileSync(join(dir, "README.md"), "# fon\n\nHuman-confirmed wildfire detection.\n");

    const s = await stub(THESIS_CLEAN);
    try {
      const out = await runCli(dir, s.url, ["thesis"], { PATH: `${bin}:${process.env.PATH}` });
      assert.equal(out.code, 0, out.stderr);

      const body = s.captured.bodies[0];
      const asked = Object.keys(body.questions);

      // `disagrees` is not declared in the payload, so the strawman question
      // is not asked at all — the same discipline as dropping `bare_quantity`
      // with no numeric candidates.
      assert.ok(!asked.includes("dissent_is_a_strawman"), asked.join(", "));
      assert.ok(asked.includes("falsifier_does_not_falsify"), asked.join(", "));
      assert.ok(asked.includes("overclaims_against_evidence"), asked.join(", "));

      // The thesis pass ships the claim and the copy, never the working diff.
      assert.equal(body.state.claim, "No unverified alert ever reaches a responder.");
      assert.ok(body.state.restatements.some((r) => r.includes("Human-confirmed")));
      assert.equal(body.state.diff, undefined);
      assert.equal(body.state.numeric_candidates, undefined);

      assert.match(out.stdout, /nothing to flag in this product's thesis/);
    } finally {
      await s.close();
    }
  });

  it("reports an overclaim first and still exits 0", async () => {
    const { dir, bin } = thesisRepo(THESIS_PAYLOAD);
    writeFileSync(join(dir, "README.md"), "# fon\n\nDeployed across Serbia today.\n");

    const dirty = {
      ...THESIS_CLEAN,
      answers: {
        ...THESIS_CLEAN.answers,
        overclaims_against_evidence: { type: "noul", noul: 0.94 },
        nobody_disagrees: { type: "noul", noul: 0.61 },
      },
    };

    const s = await stub(dirty);
    try {
      const out = await runCli(dir, s.url, ["thesis"], { PATH: `${bin}:${process.env.PATH}` });
      assert.equal(out.code, 0, "advisory output never fails the caller by default");

      const firstFinding = out.stdout.indexOf("public copy may claim more");
      const secondFinding = out.stdout.indexOf("nobody would argue with");
      assert.ok(firstFinding !== -1 && secondFinding !== -1, out.stdout);
      assert.ok(firstFinding < secondFinding, `overclaim must rank first:\n${out.stdout}`);

      // And it arrives with the offending sentence attached.
      assert.match(out.stdout, /Deployed across Serbia today/);
    } finally {
      await s.close();
    }
  });

  it("says so, sends nothing, and exits 0 when no thesis is declared", async () => {
    const { dir, bin } = thesisRepo({ declared: false });
    const s = await stub(THESIS_CLEAN);
    try {
      const out = await runCli(dir, s.url, ["thesis"], { PATH: `${bin}:${process.env.PATH}` });
      assert.equal(out.code, 0);
      assert.equal(s.captured.count, 0, "a product without a thesis sends nothing");
      assert.match(out.stdout, /no thesis declared yet/);
    } finally {
      await s.close();
    }
  });
});
