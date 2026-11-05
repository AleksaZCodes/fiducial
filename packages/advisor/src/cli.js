#!/usr/bin/env node
/**
 * `fid-advise` — the advisory surface. Asks; never decides.
 *
 * # The one invariant
 *
 * This command **cannot change anything**. It writes no file, touches no
 * artifact, and exits 0 even when it has findings — including when the network
 * is down, the key is missing, or the model is overloaded. That is not
 * defensive coding, it is the contract: `crates/fiducial-cli/tests/determinism.rs`
 * requires `fid derive` to be byte-identical with and without a key, and the
 * only way a probabilistic model can live in this platform without putting that
 * guarantee at risk is by never being in a position to affect an outcome.
 *
 * So: deterministic checks decide (`fid derive --check`, `fid guard-check`,
 * `fid doctor`). This one talks. Use `--strict` to opt into a non-zero exit,
 * for a human who wants it in a pre-commit hook of their own choosing — never
 * wire it into a gate that must not flake.
 *
 * # Why a model is involved at all
 *
 * Every question asked here is one code cannot answer. See `checks.js` for the
 * line, including the table of rules that are deliberately *not* asked because
 * an exact check already exists. The rule is: never spend a probabilistic
 * answer where a certain one is available.
 *
 * # Usage
 *
 *   fid-advise diff [--base <ref>] [--task "<text>"] [--json]   (via: fid advise)
 *   fid-advise thesis [--json] [--dry-run]                     (via: fid advise thesis)
 *   fid-advise facts [--json]                                  (via: fid advise --facts)
 *   fid-advise status                                          (via: fid advise status)
 */

import { OpenRouterSystemOne, TypeSafeSystemOne } from "@fiducial/adapters/system-one";
import {
  DIFF_QUESTIONS,
  THESIS_QUESTIONS,
  THESIS_WEIGHTS,
  duplicateOfQuestion,
  mergeFindings,
  weigh,
} from "./checks.js";
import {
  addedLines,
  collectDiff,
  collectRules,
  collectThesis,
  derivedOutputs,
  diffStat,
  docsDrift,
  existingDeclarations,
  isDerivedPath,
  mapLimit,
  numericCandidates,
  repoFiles,
  restatements,
  splitDiff,
  untouchedPlatformFiles,
} from "./gather.js";
import { credentialsPath, resolveKey } from "./key.js";
import { AGENT_BRIEF, REVIEW_DEFAULT_MODEL, reviewAll } from "./review.js";

/**
 * How much of a change one request carries. System One answers every question
 * about each unit, so its units are smaller; the reasoning model has a very
 * large window and is given more at once so related files are read together.
 */
const SYSTEM_ONE_UNIT_CHARS = 40_000;
const REVIEW_UNIT_CHARS = 120_000;
/** Requests in flight at once, per model. */
const CONCURRENCY = 4;

const args = process.argv.slice(2);
const command = args[0] ?? "diff";
const flag = (name) => {
  const i = args.indexOf(`--${name}`);
  return i === -1 ? undefined : (args[i + 1] ?? "");
};
const has = (name) => args.includes(`--${name}`);

const json = has("json");
const strict = has("strict");
const cwd = process.cwd();

/**
 * Build the configured vendor, or report why we cannot.
 *
 * `FIDUCIAL_ADVISE_ENDPOINT` redirects every request to another URL. It exists
 * so the live path — call, parse, weigh, print — can be exercised end to end
 * against a local stub, which is otherwise the one stretch of this package no
 * test reaches. It is also the hook for an egress proxy.
 *
 * It is applied by wrapping `fetch`, not by adding a base-URL option to the
 * adapter: the adapter already accepts a `fetchImpl` for exactly this, so the
 * override stays a property of the caller and the contract keeps naming the two
 * real endpoints and nothing else.
 */
function client() {
  const resolved = resolveKey();
  if (!resolved.vendor) return { error: resolved.reason };
  const model = process.env.FIDUCIAL_SYSTEMONE_MODEL || undefined;
  const endpoint = process.env.FIDUCIAL_ADVISE_ENDPOINT;
  const fetchImpl = endpoint ? (_url, init) => fetch(endpoint, init) : fetch;
  try {
    const impl =
      resolved.vendor === "openrouter"
        ? new OpenRouterSystemOne(resolved.env, model, fetchImpl)
        : new TypeSafeSystemOne(resolved.env, model, fetchImpl);
    return { impl, source: resolved.source, vendor: resolved.vendor, env: resolved.env, fetchImpl };
  } catch (err) {
    return { error: err.message };
  }
}

/** Advisory output never fails the caller unless they asked it to. */
function finish(findings) {
  if (strict && findings.some((f) => f.firm)) process.exit(1);
  process.exit(0);
}

function printFindings(findings, meta) {
  const reviewFindings = meta.review?.findings ?? [];

  if (json) {
    const { review, ...rest } = meta;
    console.log(
      JSON.stringify(
        {
          findings,
          ...rest,
          ...(review
            ? {
                review: {
                  model: review.model,
                  findings: review.findings,
                  skipped: review.skipped,
                  requests: review.requests,
                },
              }
            : {}),
          ...(meta.units !== undefined ? { next_steps: AGENT_BRIEF } : {}),
        },
        null,
        2,
      ),
    );
    return;
  }

  const notes = () => {
    if (meta.omitted?.length) {
      console.log(`  note: ${meta.omitted.length} file(s) too large to review: ${meta.omitted.join(", ")}`);
    }
    if (meta.skippedFiles?.length) {
      console.log(`  skipped on purpose (derived output, opaque, or an unmodified platform file): ${meta.skippedFiles.join(", ")}`);
    }
    for (const f of meta.failedUnits ?? []) {
      console.log(`  not reviewed by system one: ${f.unit} (${f.reason})`);
    }
    for (const s of meta.review?.skipped ?? []) {
      console.log(`  not reviewed by the reasoning model: ${s.unit} (${s.reason})`);
    }
  };

  if (findings.length === 0 && reviewFindings.length === 0) {
    console.log(`✦ fid advise — nothing to flag in ${meta.scope}`);
    notes();
    return;
  }

  const total = findings.length + reviewFindings.length;
  console.log(`✦ fid advise — ${total} thing(s) worth a look in ${meta.scope}`);
  console.log();

  if (findings.length > 0) {
    console.log("  Judgments (system one, calibrated)");
    console.log();
  }
  for (const f of findings) {
    // The probability is printed, not hidden behind a word like "warning".
    // These answers are calibrated, so the number is information the reader can
    // actually use — and it makes clear this is a second opinion, not a verdict.
    const mark = f.firm ? "▲" : "·";
    console.log(`  ${mark} ${f.title}  [${(f.probability * 100).toFixed(0)}%]`);
    console.log(`    ${f.advice}`);
    if (f.units?.length > 1) {
      console.log(`    ↳ raised in ${f.units.length} units: ${f.units.slice(0, 3).join(", ")}`);
    }
    if (f.evidence?.length) {
      for (const e of f.evidence.slice(0, 4)) console.log(`    ↳ ${e}`);
    }
    console.log();
  }

  if (reviewFindings.length > 0) {
    console.log(`  Review (${meta.review.model ?? "reasoning model"}, against this repo's rules)`);
    console.log();
    for (const f of reviewFindings) {
      const mark = f.severity === "fix" ? "▲" : "·";
      console.log(`  ${mark} ${f.file}${f.line ? `:${f.line}` : ""}${f.rule ? `  [${f.rule}]` : ""}`);
      console.log(`    ${f.problem}`);
      if (f.fix) console.log(`    fix: ${f.fix}`);
      console.log();
    }
  }

  console.log("  Advisory only — nothing here blocks anything. `▲` is a firm read, `·` something to check.");
  notes();
  if (meta.cost !== undefined) {
    console.log(`  cost: $${meta.cost.toFixed(6)} · models: ${[meta.model, meta.review?.model].filter(Boolean).join(", ")}`);
  }
  if (meta.units !== undefined) {
    console.log();
    for (const line of AGENT_BRIEF.split("\n")) console.log(`  ${line}`);
  }
}

// ── status ────────────────────────────────────────────────────────────────────

if (command === "status") {
  const resolved = resolveKey();
  console.log("✦ fid advise — status");
  console.log();
  if (resolved.vendor) {
    console.log(`  key      found — via ${resolved.source}`);
    console.log(`  route    ${resolved.vendor}`);
    console.log(`  unlocked fid advise · fid advise --facts`);
  } else {
    console.log("  key      not configured");
    console.log(`  file     ${credentialsPath()}`);
    console.log("  set it   fid advise key set");
    console.log();
    console.log("  Advisory checks are off. Every deterministic check is unaffected —");
    console.log("  `fid derive`, `fid doctor` and the guard behave identically either way.");
  }
  process.exit(0);
}

// ── diff ──────────────────────────────────────────────────────────────────────

if (command === "diff") {
  const { diff: raw, files, scope } = collectDiff(cwd, flag("base"));

  if (!raw.trim()) {
    if (json) console.log(JSON.stringify({ findings: [], scope, empty: true }));
    else console.log(`✦ fid advise — no changes in ${scope}`);
    process.exit(0);
  }

  // `--dry-run` resolves no key and sends nothing. It exists because this
  // command ships your diff to a third-party API, and "trust me" is not an
  // acceptable answer to what exactly leaves the machine. It also makes the
  // deterministic half — which is most of the code — debuggable offline.
  const dryRun = has("dry-run");

  const c = dryRun ? {} : client();
  if (c.error) {
    // Not an error exit: "no key" is a normal state and this runs in a hook.
    if (json) console.log(JSON.stringify({ findings: [], skipped: c.error }));
    else console.log(`✦ fid advise — skipped: ${c.error}`);
    process.exit(0);
  }

  const task = flag("task");
  const declarations = existingDeclarations(cwd);

  // The change is reviewed in structural units, by both models, so neither is
  // shown a truncated slice and a large change costs more requests instead of
  // a worse review. System One gets smaller units than the reasoning model: it
  // answers every question about each one, so its input is what bounds it.
  const untouched = untouchedPlatformFiles(cwd);
  const derived = derivedOutputs(cwd);
  const skip = (name) => untouched.has(name) || derived.has(name);
  const { units, skipped: skippedFiles } = splitDiff(raw, SYSTEM_ONE_UNIT_CHARS, { skip });

  const unitState = (unit) => {
    const candidates = numericCandidates(addedLines(unit.diff));
    // A question whose input is missing is not asked: an answer about an empty
    // list, or about "what was asked" with nothing asked, is noise.
    const questions = { ...DIFF_QUESTIONS };
    if (candidates.length === 0) delete questions.bare_quantity;
    if (!task) delete questions.unrequested_scope;
    return {
      candidates,
      questions,
      state: {
        ...(task ? { task } : {}),
        changed_files: files,
        unit_files: unit.files,
        diff: unit.diff,
        numeric_candidates: candidates.map((x) => `${x.file}:${x.line} ${x.name} = ${x.value}`),
        existing_declarations: declarations,
      },
    };
  };

  const rules = collectRules(cwd);
  const docs = docsDrift(cwd, files, raw, { skip });
  const stat = diffStat(raw).filter((s) => !isDerivedPath(s.file) && !skip(s.file));
  const reviewUnits = splitDiff(raw, REVIEW_UNIT_CHARS, { skip }).units;
  const repo = repoFiles(cwd);
  const reviewModel = process.env.FIDUCIAL_REVIEW_MODEL || REVIEW_DEFAULT_MODEL;
  // The reasoning model is reached through OpenRouter's chat endpoint, so it
  // needs that key. A TypeSafe-only key still gets System One.
  const reviewing = !has("no-review") && (dryRun || c.vendor === "openrouter");

  if (dryRun) {
    const plan = units.map((u) => ({ id: u.id, label: u.label, files: u.files, chars: u.diff.length, ...unitState(u) }));
    if (json) {
      console.log(
        JSON.stringify(
          {
            units: plan,
            skipped_files: skippedFiles,
            review: reviewing
              ? { model: reviewModel, requests: reviewUnits.length + 1, docs, rules: Object.keys(rules) }
              : null,
          },
          null,
          2,
        ),
      );
    } else {
      console.log(`✦ fid advise --dry-run — nothing sent. This is the plan.`);
      console.log();
      console.log(`  scope             ${scope}`);
      console.log(`  changed files     ${files.length}`);
      console.log(`  skipped files     ${skippedFiles.length ? skippedFiles.join(", ") : "none"}`);
      console.log(`  system one        ${units.length} request(s), one per unit`);
      for (const u of plan) {
        console.log(`    ↳ ${u.label}  ${u.chars} chars  ${Object.keys(u.questions).length} questions`);
      }
      if (reviewing) {
        console.log(`  review model      ${reviewModel}`);
        console.log(`    ↳ ${reviewUnits.length} unit request(s) + 1 overview`);
        console.log(`    ↳ rules sent: ${Object.keys(rules).join(", ") || "none"}`);
        console.log(`    ↳ docs that may be stale: ${docs.stale.length}, undocumented flags: ${docs.undocumented.length}`);
      } else {
        console.log(`  review model      off (--no-review, or no OpenRouter key)`);
      }
      console.log();
      console.log("  Add --json to see every unit's request verbatim.");
    }
    process.exit(0);
  }

  // Every unit is asked at once, a few at a time. Questions inside a request
  // are evaluated in parallel and in isolation, and units are independent, so
  // none of them can bias another.
  const sysOne = mapLimit(units, CONCURRENCY, async (unit) => {
    const { state, questions, candidates } = unitState(unit);
    try {
      const response = await c.impl.decide({
        state,
        questions,
        sessionId: `advise-diff-${Date.now()}-${unit.id}`,
      });
      return { unit, candidates, response };
    } catch (err) {
      return { unit, candidates, error: String(err.message ?? err) };
    }
  });

  const reasoning = reviewing
    ? reviewAll({
        apiKey: c.env.OPENROUTER_API_KEY,
        fetchImpl: c.fetchImpl,
        model: reviewModel,
        units: reviewUnits,
        stat,
        docs,
        files,
        repo,
        rules,
        task,
        concurrency: CONCURRENCY,
      })
    : Promise.resolve(null);

  const [answered, reviewed] = await Promise.all([sysOne, reasoning]);

  const worked = answered.filter((a) => a.response);
  const failed = answered.filter((a) => a.error);
  // A model that is down must never be the reason a commit cannot happen — but
  // when *neither* answered, the report says so instead of claiming all clear.
  if (worked.length === 0 && (!reviewed || reviewed.findings.length === 0) && (failed.length > 0 || reviewed?.skipped.length)) {
    const why = failed[0]?.error ?? reviewed?.skipped[0]?.reason ?? "no model answered";
    if (json) console.log(JSON.stringify({ findings: [], skipped: why }));
    else console.log(`✦ fid advise — skipped: ${why}`);
    process.exit(0);
  }

  const findings = mergeFindings(
    worked.map((a) => ({ label: a.unit.label, findings: weigh(a.response.answers) })),
  );

  // Attach the mechanically-found evidence, so a finding points at real lines
  // rather than asking the reader to trust a description.
  const allCandidates = worked.flatMap((a) => a.candidates);
  for (const f of findings) {
    if (f.key === "bare_quantity") {
      f.evidence = allCandidates.map((x) => `${x.file}:${x.line}  ${x.name} = ${x.value}`);
    }
  }

  const cost =
    worked.reduce((sum, a) => sum + (a.response.usage.cost ?? 0), 0) + (reviewed?.cost ?? 0);

  printFindings(findings, {
    scope,
    skippedFiles,
    units: units.length,
    failedUnits: failed.map((f) => ({ unit: f.unit.label, reason: f.error })),
    review: reviewed,
    docs,
    cost,
    model: worked[0]?.response.model,
  });
  finish([...findings, ...(reviewed?.findings ?? []).map((f) => ({ ...f, firm: f.severity === "fix" }))]);
}

// ── facts ─────────────────────────────────────────────────────────────────────

if (command === "thesis") {
  // The declaration is read through `fid thesis --json`, so this command and
  // the platform never disagree about which entry is current.
  const thesis = collectThesis(cwd);
  if (!thesis) {
    // No thesis is a normal state, not a failure. A product is supposed to be
    // able to exist before it has decided what it claims, and saying so is
    // more useful than an error about a missing file.
    if (json) console.log(JSON.stringify({ findings: [], scope: "thesis", empty: true }));
    else {
      console.log("✦ fid advise — no thesis declared yet");
      console.log();
      console.log('  Declare one with `fid thesis set "<claim>"`. One rough line is enough;');
      console.log("  this command is for sharpening it, not for starting it.");
    }
    process.exit(0);
  }

  const dryRun = has("dry-run");
  const c = dryRun ? {} : client();
  if (c.error) {
    if (json) console.log(JSON.stringify({ findings: [], skipped: c.error }));
    else console.log(`✦ fid advise — skipped: ${c.error}`);
    process.exit(0);
  }

  const current = thesis.current ?? {};
  const copy = restatements(cwd);
  const notYet = current.evidence?.not_yet ?? [];

  // A question whose input is missing is not asked — the same discipline that
  // drops `bare_quantity` with no numeric candidates. Every one of these has a
  // required input that an early thesis simply will not have yet, and asking
  // anyway would mean judging an empty string and reporting the result as
  // advice about the product.
  const questions = { ...THESIS_QUESTIONS };
  if (!current.test?.falsified_by) delete questions.falsifier_does_not_falsify;
  if (!current.test?.disagrees) delete questions.dissent_is_a_strawman;
  if (copy.length === 0) {
    delete questions.copy_contradicts_the_claim;
    delete questions.overclaims_against_evidence;
  } else if (notYet.length === 0) {
    // Copy exists but the thesis never says what is unproven, so there is
    // nothing to measure an overclaim against. `fid thesis` already asks for
    // `evidence.not_yet`; inventing a standard here would be worse than silence.
    delete questions.overclaims_against_evidence;
  }

  const state = {
    claim: current.claim,
    ...(current.arc?.problem ? { problem: current.arc.problem } : {}),
    ...(current.arc?.for_whom ? { for_whom: current.arc.for_whom } : {}),
    ...(current.test?.falsified_by ? { falsified_by: current.test.falsified_by } : {}),
    ...(current.test?.disagrees ? { disagrees: current.test.disagrees } : {}),
    ...(notYet.length ? { evidence_not_yet: notYet } : {}),
    ...(copy.length ? { restatements: copy } : {}),
  };

  if (dryRun) {
    const body = { state, questions };
    if (json) {
      console.log(JSON.stringify(body, null, 2));
    } else {
      console.log("✦ fid advise thesis --dry-run — nothing sent. This is the exact request body.");
      console.log();
      console.log(`  claim             ${current.claim}`);
      console.log(`  questions asked   ${Object.keys(questions).join(", ")}`);
      const skipped = Object.keys(THESIS_QUESTIONS).filter((k) => !(k in questions));
      console.log(`  not asked         ${skipped.length ? skipped.join(", ") : "none"}`);
      console.log(`  restatements      ${copy.length}`);
      for (const r of copy) console.log(`    ↳ ${r.slice(0, 96)}`);
      console.log(`  unanswered fields ${(thesis.gaps ?? []).map((g) => g.field).join(", ") || "none"}`);
      console.log();
      console.log("  Add --json to see the full body verbatim.");
    }
    process.exit(0);
  }

  let response;
  try {
    response = await c.impl.decide({
      state,
      questions,
      sessionId: `advise-thesis-${Date.now()}`,
    });
  } catch (err) {
    if (json) console.log(JSON.stringify({ findings: [], skipped: String(err.message ?? err) }));
    else console.log(`✦ fid advise — skipped: ${err.message ?? err}`);
    process.exit(0);
  }

  const findings = weigh(response.answers, { weights: THESIS_WEIGHTS });

  // Point the copy findings at the actual sentences, so "your copy contradicts
  // the claim" arrives with the copy attached rather than as a description of
  // something the model believes it read.
  for (const f of findings) {
    if (f.key === "copy_contradicts_the_claim" || f.key === "overclaims_against_evidence") {
      f.evidence = copy;
    }
  }

  printFindings(findings, {
    scope: "this product's thesis",
    cost: response.usage.cost,
    model: response.model,
  });
  finish(findings);
}

if (command === "facts") {
  const c = client();
  if (c.error) {
    if (json) console.log(JSON.stringify({ findings: [], skipped: c.error }));
    else console.log(`✦ fid advise — skipped: ${c.error}`);
    process.exit(0);
  }

  const { diff: raw } = collectDiff(cwd, flag("base"));
  const declarations = existingDeclarations(cwd);
  const added = addedLines(raw);

  // Candidates are declarations *being added*. Checking every existing
  // declaration against every other is quadratic and mostly re-answers
  // questions nobody changed; what matters is whether what you just wrote
  // already existed.
  const candidates = [];
  for (const { file, line, text } of added) {
    const m = /^\s*([A-Za-z_][A-Za-z0-9_.\-]*)\s*=\s*(.+)$/.exec(text);
    if (!m) continue;
    const [, name, value] = m;
    if (value.trim().startsWith("[") || value.trim().startsWith("{")) continue;
    candidates.push({ file, line, name, value: value.trim().slice(0, 80) });
    if (candidates.length >= 12) break;
  }

  if (candidates.length === 0) {
    if (json) console.log(JSON.stringify({ findings: [], scope: "new declarations" }));
    else console.log("✦ fid advise — no new declarations to check for duplicates");
    process.exit(0);
  }

  // One Choice per candidate against the existing set as options: linear in
  // candidates, and the answer arrives with a probability for every existing
  // declaration rather than a bare yes/no. Capped at the API's 255 options.
  const options = Object.fromEntries(Object.entries(declarations).slice(0, 250));
  const questions = {};
  for (const [i, cand] of candidates.entries()) {
    questions[`dup_${i}`] = duplicateOfQuestion(`${cand.name} = ${cand.value}`, options);
  }

  let response;
  try {
    response = await c.impl.decide({
      state: { declared_in_repository: options },
      questions,
      sessionId: `advise-facts-${Date.now()}`,
    });
  } catch (err) {
    if (json) console.log(JSON.stringify({ findings: [], skipped: String(err.message ?? err) }));
    else console.log(`✦ fid advise — skipped: ${err.message ?? err}`);
    process.exit(0);
  }

  const findings = [];
  for (const [i, cand] of candidates.entries()) {
    const a = response.answers[`dup_${i}`];
    if (!a || a.type !== "choice") continue;
    if (a.choice === "none_of_these") continue;
    const p = a.probabilities?.[a.choice] ?? 0;
    if (p < 0.55 || (a.confidence ?? 0) < 0.6) continue;
    findings.push({
      key: "duplicate_declaration",
      title: `\`${cand.name}\` may already be declared as \`${a.choice}\``,
      advice: "Derive from the existing declaration instead of restating it.",
      probability: p,
      firm: p >= 0.8,
      score: p,
      evidence: [`${cand.file}:${cand.line}  ${cand.name} = ${cand.value}`],
    });
  }

  printFindings(findings.sort((a, b) => b.score - a.score), {
    scope: `${candidates.length} new declaration(s)`,
    cost: response.usage.cost,
    model: response.model,
  });
  finish(findings);
}

console.error(`fid-advise: unknown command \`${command}\`

  fid-advise diff [--base <ref>] [--task "<what was asked>"] [--json] [--strict]
  fid-advise thesis [--json] [--dry-run]
  fid-advise facts [--json]
  fid-advise status`);
process.exit(2);
