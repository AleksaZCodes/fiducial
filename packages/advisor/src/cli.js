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
  weigh,
} from "./checks.js";
import {
  addedLines,
  collectDiff,
  collectThesis,
  existingDeclarations,
  fitDiff,
  numericCandidates,
  restatements,
} from "./gather.js";
import { credentialsPath, resolveKey } from "./key.js";

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
    return { impl, source: resolved.source, vendor: resolved.vendor };
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
  if (json) {
    console.log(JSON.stringify({ findings, ...meta }, null, 2));
    return;
  }

  if (findings.length === 0) {
    console.log(`✦ fid advise — nothing to flag in ${meta.scope}`);
    if (meta.omitted?.length) {
      console.log(`  note: ${meta.omitted.length} file(s) too large to review: ${meta.omitted.join(", ")}`);
    }
    return;
  }

  console.log(`✦ fid advise — ${findings.length} thing(s) worth a look in ${meta.scope}`);
  console.log();
  for (const f of findings) {
    // The probability is printed, not hidden behind a word like "warning".
    // These answers are calibrated, so the number is information the reader can
    // actually use — and it makes clear this is a second opinion, not a verdict.
    const mark = f.firm ? "▲" : "·";
    console.log(`  ${mark} ${f.title}  [${(f.probability * 100).toFixed(0)}%]`);
    console.log(`    ${f.advice}`);
    if (f.evidence?.length) {
      for (const e of f.evidence.slice(0, 4)) console.log(`    ↳ ${e}`);
    }
    console.log();
  }
  console.log("  Advisory only — nothing here blocks anything. `▲` is a firm read, `·` a hint.");
  if (meta.omitted?.length) {
    console.log(`  note: ${meta.omitted.length} file(s) too large to review: ${meta.omitted.join(", ")}`);
  }
  if (meta.cost !== undefined) {
    console.log(`  cost: $${meta.cost.toFixed(6)} · model: ${meta.model}`);
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

  const { diff, omitted } = fitDiff(raw);
  const added = addedLines(diff);
  const candidates = numericCandidates(added);
  const declarations = existingDeclarations(cwd);

  // The whole question set goes in one request: questions are evaluated in
  // parallel and in isolation, so seven cost barely more latency than one and
  // none of them can bias another.
  const questions = { ...DIFF_QUESTIONS };
  if (candidates.length === 0) {
    // Nothing mechanical to judge, so do not ask — an answer about an empty
    // list is noise, and this is the hybrid split doing its job.
    delete questions.bare_quantity;
  }

  const task = flag("task");
  if (!task) {
    // `unrequested_scope` compares the diff against what was asked. With
    // nothing to compare against it degrades into "does this diff contain
    // anything I would not have written", which a model will answer and which
    // means nothing. Same discipline as dropping `bare_quantity` with no
    // candidates: a question whose input is missing is not asked.
    delete questions.unrequested_scope;
  }

  const state = {
    ...(task ? { task } : {}),
    changed_files: files,
    diff,
    numeric_candidates: candidates.map((x) => `${x.file}:${x.line} ${x.name} = ${x.value}`),
    existing_declarations: declarations,
  };

  if (dryRun) {
    const body = { state, questions };
    if (json) {
      console.log(JSON.stringify(body, null, 2));
    } else {
      console.log(`✦ fid advise --dry-run — nothing sent. This is the exact request body.`);
      console.log();
      console.log(`  scope             ${scope}`);
      console.log(`  changed files     ${files.length}`);
      console.log(`  diff sent         ${diff.length} chars`);
      console.log(`  files omitted     ${omitted.length ? omitted.join(", ") : "none"}`);
      console.log(`  questions asked   ${Object.keys(questions).join(", ")}`);
      console.log(`  numeric candidates ${candidates.length}`);
      for (const x of candidates.slice(0, 8)) {
        console.log(`    ↳ ${x.file}:${x.line}  ${x.name} = ${x.value}`);
      }
      console.log(`  declarations sent ${Object.keys(declarations).length}`);
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
      sessionId: `advise-diff-${Date.now()}`,
    });
  } catch (err) {
    // A model that is down must never be the reason a commit cannot happen.
    if (json) console.log(JSON.stringify({ findings: [], skipped: String(err.message ?? err) }));
    else console.log(`✦ fid advise — skipped: ${err.message ?? err}`);
    process.exit(0);
  }

  const findings = weigh(response.answers);

  // Attach the mechanically-found evidence, so a finding points at real lines
  // rather than asking the reader to trust a description.
  for (const f of findings) {
    if (f.key === "bare_quantity") {
      f.evidence = candidates.map((x) => `${x.file}:${x.line}  ${x.name} = ${x.value}`);
    }
  }

  printFindings(findings, {
    scope,
    omitted,
    cost: response.usage.cost,
    model: response.model,
  });
  finish(findings);
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
