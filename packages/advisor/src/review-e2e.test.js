/**
 * End to end: both models, in units, through the real CLI.
 *
 * `endtoend.test.js` covers System One alone (it passes `--no-review`). This
 * covers what is new: the reasoning model reached beside it, the change split
 * into units for both, the repository's own rules sent along, and a report
 * shaped for the agent that ran the command.
 *
 * The vendor is one local server, because `FIDUCIAL_ADVISE_ENDPOINT` redirects
 * every request to a single URL. It tells the two apart by body: a decisions
 * request has `questions`, a chat request has `messages`.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const CLI = join(dirname(fileURLToPath(import.meta.url)), "cli.js");

const DECISIONS_CLEAN = {
  model: "typesafe/jev-1.14-20261014",
  answers: {
    misplaced_layer: {
      type: "choice",
      choice: "already_at_the_right_layer",
      probabilities: { already_at_the_right_layer: 0.95 },
      confidence: 0.93,
    },
    duplicate_declaration: { type: "noul", noul: 0.04 },
    abstraction_without_escape: { type: "noul", noul: 0.05 },
    comment_explains_what: { type: "noul", noul: 0.03 },
    half_finished: { type: "noul", noul: 0.01 },
  },
  usage: { input_tokens: 400, output_tokens: 40, cost: 0.00001 },
};

const REVIEW_FINDINGS = [
  {
    file: "README.md",
    line: 2,
    rule: "documentation went stale",
    problem: "README still documents the old flag.",
    fix: "Describe --frobnicate.",
    severity: "fix",
  },
];

function chatReply(findings) {
  return {
    model: "anthropic/claude-haiku-5.5-20261001",
    choices: [{ message: { content: JSON.stringify({ findings }) } }],
    usage: { cost: 0.002 },
  };
}

/** A repo with rules, a README, and two large new files in different directories. */
function repo() {
  const dir = mkdtempSync(join(tmpdir(), "advisor-review-"));
  const run = (args) => execFileSync("git", args, { cwd: dir, stdio: "ignore" });
  run(["init", "-q", "."]);
  run(["config", "user.email", "t@example.com"]);
  run(["config", "user.name", "t"]);
  writeFileSync(join(dir, "AGENTS.md"), "# Rules\nRULE-MARKER-ONE: never edit a derived file.\n");
  writeFileSync(join(dir, "README.md"), "# Tool\nUse `src/a/big.ts`.\n");
  run(["add", "-A"]);
  run(["commit", "-qm", "init"]);

  const body = (n) => Array.from({ length: 600 }, (_, i) => `export const v${n}_${i} = ${i}; // padding padding`).join("\n");
  mkdirSync(join(dir, "src/a"), { recursive: true });
  mkdirSync(join(dir, "src/b"), { recursive: true });
  writeFileSync(join(dir, "src/a/big.ts"), body(1));
  writeFileSync(join(dir, "src/b/big.ts"), body(2));
  return dir;
}

async function stub({ decisions = DECISIONS_CLEAN, chat = chatReply(REVIEW_FINDINGS), chatStatus = 200 } = {}) {
  const captured = { decisions: [], chat: [] };
  const server = createServer((req, res) => {
    let raw = "";
    req.on("data", (c) => (raw += c));
    req.on("end", () => {
      let body = {};
      try {
        body = JSON.parse(raw);
      } catch {
        body = {};
      }
      const isChat = Array.isArray(body.messages);
      (isChat ? captured.chat : captured.decisions).push(body);
      res.writeHead(isChat ? chatStatus : 200, { "content-type": "application/json" });
      res.end(JSON.stringify(isChat ? chat : decisions));
    });
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  return {
    url: `http://127.0.0.1:${server.address().port}/`,
    captured,
    close: () =>
      new Promise((r) => {
        server.closeAllConnections?.();
        server.close(r);
      }),
  };
}

function runCli(cwd, endpoint, args, extraEnv = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [CLI, ...args], {
      cwd,
      env: {
        ...process.env,
        OPENROUTER_API_KEY: "sk-or-v1-stub",
        TYPESAFE_API_KEY: "",
        FIDUCIAL_ADVISE_ENDPOINT: endpoint,
        FIDUCIAL_REVIEW_MODEL: "",
        ...extraEnv,
      },
    });
    let stdout = "";
    child.stdout.on("data", (c) => (stdout += c));
    child.on("error", reject);
    child.on("close", (code) => resolve({ code, stdout }));
  });
}

describe("end to end — both models, in units", () => {
  it("splits the change for both models and sends the repository's rules to the reviewer", async () => {
    const s = await stub();
    try {
      const r = await runCli(repo(), s.url, ["diff"]);
      assert.equal(r.code, 0);

      assert.ok(s.captured.decisions.length >= 2, "system one is asked once per unit");
      assert.ok(s.captured.chat.length >= 3, "the reviewer gets one request per unit, plus the overview");

      const first = s.captured.chat[0];
      assert.equal(first.model, "anthropic/claude-haiku-5.5");
      const text = first.messages[1].content;
      assert.match(text, /RULE-MARKER-ONE/, "the repository's own AGENTS.md is the rulebook");
      assert.match(text, /CHANGED_FILES/);
      assert.match(text, /src\/a\/big\.ts/);

      for (const body of s.captured.decisions) {
        assert.ok(JSON.stringify(body.state.diff).length < 60_000, "no unit exceeds system one's budget");
      }
    } finally {
      await s.close();
    }
  });

  it("prints both reports and tells the agent what to do with them", async () => {
    const s = await stub();
    try {
      const r = await runCli(repo(), s.url, ["diff"]);
      assert.match(r.stdout, /Review \(anthropic\/claude-haiku-5\.5-20261001/);
      assert.match(r.stdout, /README\.md:2/);
      assert.match(r.stdout, /README still documents the old flag/);
      assert.match(r.stdout, /fix: Describe --frobnicate/);
      assert.match(r.stdout, /claim to check/);
      assert.match(r.stdout, /Never edit docs\/specs\//);
    } finally {
      await s.close();
    }
  });

  it("gives the agent the same thing as data under --json", async () => {
    const s = await stub();
    try {
      const r = await runCli(repo(), s.url, ["diff", "--json"]);
      const out = JSON.parse(r.stdout);
      assert.equal(out.review.findings[0].file, "README.md");
      assert.equal(out.review.model, "anthropic/claude-haiku-5.5-20261001");
      assert.match(out.next_steps, /claim to check/);
      assert.ok(out.units >= 2);
    } finally {
      await s.close();
    }
  });

  it("asks the reasoning model nothing under --no-review", async () => {
    const s = await stub();
    try {
      const r = await runCli(repo(), s.url, ["diff", "--no-review"]);
      assert.equal(r.code, 0);
      assert.equal(s.captured.chat.length, 0);
      assert.ok(s.captured.decisions.length >= 2);
      assert.doesNotMatch(r.stdout, /Review \(/);
    } finally {
      await s.close();
    }
  });

  it("does not send a chat request when the key is TypeSafe-only", async () => {
    const s = await stub();
    try {
      const r = await runCli(repo(), s.url, ["diff"], { OPENROUTER_API_KEY: "", TYPESAFE_API_KEY: "tk-stub" });
      assert.equal(r.code, 0);
      assert.equal(s.captured.chat.length, 0);
      assert.ok(s.captured.decisions.length >= 2);
    } finally {
      await s.close();
    }
  });

  it("honours FIDUCIAL_REVIEW_MODEL", async () => {
    const s = await stub();
    try {
      await runCli(repo(), s.url, ["diff"], { FIDUCIAL_REVIEW_MODEL: "anthropic/claude-haiku-4.5" });
      assert.equal(s.captured.chat[0].model, "anthropic/claude-haiku-4.5");
    } finally {
      await s.close();
    }
  });

  it("still reports system one's findings, and says so, when the reviewer is down", async () => {
    const dirty = {
      ...DECISIONS_CLEAN,
      answers: { ...DECISIONS_CLEAN.answers, duplicate_declaration: { type: "noul", noul: 0.93 } },
    };
    const s = await stub({ decisions: dirty, chat: { error: { message: "overloaded" } }, chatStatus: 503 });
    try {
      const r = await runCli(repo(), s.url, ["diff"]);
      assert.equal(r.code, 0);
      assert.match(r.stdout, /declared twice/);
      assert.match(r.stdout, /not reviewed by the reasoning model/);
      assert.match(r.stdout, /overloaded/);
    } finally {
      await s.close();
    }
  });

  it("shows the plan, sends nothing, under --dry-run", async () => {
    const s = await stub();
    try {
      const r = await runCli(repo(), s.url, ["diff", "--dry-run"]);
      assert.equal(r.code, 0);
      assert.equal(s.captured.chat.length + s.captured.decisions.length, 0);
      assert.match(r.stdout, /nothing sent/);
      assert.match(r.stdout, /anthropic\/claude-haiku-5\.5/);
      assert.match(r.stdout, /rules sent: AGENTS\.md/);
    } finally {
      await s.close();
    }
  });
});
