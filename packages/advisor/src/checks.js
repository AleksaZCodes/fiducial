/**
 * The advisory question set — declared once, here.
 *
 * These questions are **facts** in this platform's sense, so they live in one
 * place and every consumer derives from them: the CLI, the Claude Code hook,
 * and anything else that wants a second opinion. Writing a question inline at a
 * call site would be declaring the same fact twice.
 *
 * # What belongs here, and what emphatically does not
 *
 * A question earns a place here only when **code cannot answer it**. That line
 * is the whole design, so it is worth stating both sides.
 *
 * Already answered deterministically, and therefore absent on purpose:
 *
 * | Rule | What answers it |
 * |---|---|
 * | An artifact was hand-edited | `fid derive --check` — a hash comparison |
 * | A translation is missing | the i18n pipeline — a key-set difference |
 * | Contrast ratios are met | `fid-design` — arithmetic |
 * | A cost ceiling is exceeded | the cost assertion — arithmetic |
 * | Someone pushed to main | the guard's `no-direct-main-push` rule |
 *
 * Asking a model any of those would be slower, cost money, and replace a
 * certain answer with a probable one. A probabilistic check where an exact one
 * exists is a regression dressed as a feature.
 *
 * What remains is the set below: judgments about *meaning*. A regex can see
 * that two constants hold `1.6`; it cannot see that `board_thickness` and
 * `pcb_height` are one fact declared twice, which is the precise violation this
 * platform exists to prevent. Encoding that in rules is not merely hard, it is
 * the wrong shape — every rule added makes the next false positive weirder.
 *
 * # Hybrid checks: code narrows, the model judges
 *
 * `bare_quantity` is the pattern worth copying. Distinguishing a physical fact
 * that needs a unit and tolerance from a retry count or an array index is a
 * judgment; *finding candidate numeric literals* is a regex. So code does the
 * cheap over-inclusive pass and the model judges only what survives. The
 * deterministic half stays deterministic and the expensive half stays small.
 *
 * # Shape rules these questions follow
 *
 * - One narrow judgment each. An omnibus "is this good?" is not answerable in
 *   the snap-judgment way these models are built for, and its answer is not
 *   actionable. Severity is composed in `weigh()` from the individual answers,
 *   with the weights in code where they can be changed without rewriting a
 *   prompt.
 * - Every Choice carries an escape option. A model answers only within the
 *   options given, so omitting "none of these" forces a wrong answer where an
 *   honest one was available.
 * - Nouls are phrased so that **true means a problem**. Mixed polarity across a
 *   question set is how a threshold ends up inverted, and an inverted advisory
 *   check is worse than no check: it reports clean diffs and stays quiet on bad
 *   ones.
 */

/** Fiducial's reach layers, used as Choice criteria. From ARCHITECTURE.md §3. */
const LAYERS = {
  l0_rust_core:
    "Pure logic with no platform dependency: facts, quantities, units, " +
    "protocol encoding, geometry, math, state machines. Reaches browser, " +
    "desktop, edge, Node, and microcontrollers.",
  l1_tokens:
    "Design tokens: CSS custom properties, a Tailwind preset, 3D material " +
    "definitions. Consumed by any web framework.",
  l2_headless_ts:
    "Framework-agnostic TypeScript: transports, API clients, WASM loading, " +
    "offline queues. Reaches any JS framework but not a microcontroller.",
  l3_framework_ui:
    "Components tied to one UI framework — JSX, Svelte components, framework " +
    "hooks or lifecycle. Survives only until the framework changes.",
  l4_repo_system:
    "Repository tooling: the CLI, guard, codemods, the derive runner, CI " +
    "config, templates. Governs repositories rather than running in a product.",
  already_at_the_right_layer:
    "The code is already at the lowest layer it could reach, or the question " +
    "does not apply to this change.",
};

/**
 * Questions asked about a working diff.
 *
 * This is the "is the agent's output actually sound" pass. It runs on a Claude
 * Code Stop hook and before a commit, where the cost of a wrong answer is a
 * sentence a human reads and the cost of a missed one is a rule quietly broken.
 */
export const DIFF_QUESTIONS = {
  misplaced_layer: {
    type: "choice",
    instructions:
      "Principle 2 of this platform says to push behavior down to the layer " +
      "with the longest reach: logic in a pure Rust core runs everywhere, " +
      "logic in a UI framework lasts until the framework changes. Looking at " +
      "the code added in `diff`, what is the LOWEST layer the bulk of its " +
      "logic could correctly live at? Judge where it *could* live, not where " +
      "it currently sits.",
    criteria: LAYERS,
  },

  duplicate_declaration: {
    type: "noul",
    instructions:
      "Principle 1 says each fact is declared exactly once and every other " +
      "use is derived from that declaration. Does `diff` introduce a value, " +
      "constant, string, or configuration entry that duplicates something " +
      "already declared elsewhere in `diff` or named in `existing_declarations`?",
    criteria: {
      true:
        "The same fact is now stated in two places — a literal repeated, a " +
        "constant redefined, or a value hardcoded that already has a " +
        "declaration it should be derived from.",
      false:
        "Every value introduced is declared once, or is a genuinely new fact " +
        "with no existing declaration to derive from.",
    },
  },

  bare_quantity: {
    type: "noul",
    instructions:
      "This platform holds that a physical quantity is not a fact until it " +
      "carries a unit and a tolerance: `1.6` is not a fact, `1.6 mm ± 10%` " +
      "is. The numeric literals in `numeric_candidates` were found " +
      "mechanically and most will be ordinary programming values. Does at " +
      "least one of them represent a PHYSICAL quantity — a length, mass, " +
      "voltage, current, time, temperature, frequency, or force — declared " +
      "as a bare number?",
    criteria: {
      true:
        "At least one literal is a physical measurement that should carry a " +
        "unit and a tolerance.",
      false:
        "They are all ordinary programming values: counts, indices, retry " +
        "limits, timeouts used as timeouts, version numbers, HTTP statuses, " +
        "array sizes, or bit masks.",
    },
  },

  abstraction_without_escape: {
    type: "noul",
    instructions:
      "Principle 3 says no abstraction without an escape hatch: a caller must " +
      "be able to drop to a lower level without abandoning the system. Does " +
      "`diff` add a wrapper, helper, or abstraction that hides something a " +
      "caller may legitimately need, with no documented way to reach past it?",
    criteria: {
      true:
        "A new abstraction narrows what callers can do and offers no override, " +
        "no access to the underlying value, and no documented escape.",
      false:
        "No new abstraction, or the abstraction leaves the lower level " +
        "reachable.",
    },
  },

  comment_explains_what: {
    type: "noul",
    instructions:
      "This codebase's rule for comments is that they explain WHY — a hidden " +
      "constraint, a subtle invariant, a workaround, something that would " +
      "surprise a reader. A comment restating what well-named code already " +
      "says is noise that rots. Does `diff` add at least one comment that " +
      "merely describes what the code does?",
    criteria: {
      true:
        "A comment restates the code, names the current task or ticket, or " +
        "describes which callers use it — all of which the code or the commit " +
        "message already carries.",
      false:
        "Comments added explain a reason, a constraint, or a surprise; or no " +
        "comments were added.",
    },
  },

  unrequested_scope: {
    type: "noul",
    instructions:
      "Compare the work in `diff` against what `task` asked for. Does the " +
      "diff include substantial changes nobody asked for — opportunistic " +
      "refactoring, new abstractions built for hypothetical future needs, " +
      "backwards-compatibility shims, or features beyond the request?",
    criteria: {
      true:
        "Meaningful unrequested work is present: a refactor riding along with " +
        "a fix, speculative generality, or an unasked-for feature.",
      false:
        "The diff does what was asked, plus only changes genuinely required " +
        "to make that work.",
    },
  },

  half_finished: {
    type: "noul",
    instructions:
      "Does `diff` leave work in a visibly unfinished state — a stub that " +
      "returns nothing, a TODO standing in for logic the change needed, a " +
      "branch that silently does nothing, or an error swallowed to make " +
      "something pass?",
    criteria: {
      true:
        "Something is left incomplete or silently inert in a way a reader " +
        "would have to come back and finish.",
      false: "The change is complete as far as it goes.",
    },
  },
};

/**
 * How much each diff finding matters, and what to print when it fires.
 *
 * Weights live in code, not in the questions, because that is the point of
 * composing atomic judgments: when priorities shift you change a number here
 * rather than rewriting what was asked. `advice` is written as an instruction
 * to the reader, since a finding that does not say what to do is a complaint.
 */
export const DIFF_WEIGHTS = {
  duplicate_declaration: {
    weight: 1.0,
    title: "a fact may now be declared twice",
    advice:
      "Find the existing declaration and derive from it. If there is none, " +
      "make one — then derive both uses.",
  },
  misplaced_layer: {
    weight: 0.9,
    title: "logic may be placed above the layer it could reach",
    advice:
      "Moving it down costs the same to write now and reaches more runtimes " +
      "for longer. Moving it later is a migration.",
  },
  bare_quantity: {
    weight: 0.8,
    title: "a physical quantity may be declared without unit or tolerance",
    advice:
      "Give it a unit and a tolerance. Tolerance stacking across domains is " +
      "what answers 'will it fit' — nominal values cannot.",
  },
  half_finished: {
    weight: 0.7,
    title: "the change may be left unfinished",
    advice: "Finish it or remove it. A silent stub reads as working code.",
  },
  abstraction_without_escape: {
    weight: 0.6,
    title: "an abstraction may have no escape hatch",
    advice:
      "Document how to reach the lower level. The first hard problem will " +
      "find this edge.",
  },
  unrequested_scope: {
    weight: 0.5,
    title: "the diff may do more than was asked",
    advice: "Split the unrequested part out, or drop it.",
  },
  comment_explains_what: {
    weight: 0.3,
    title: "a comment may explain what instead of why",
    advice: "Delete it, or replace it with the reason the code is this way.",
  },
};

/**
 * Build the Choice question that finds a semantic duplicate of one declaration.
 *
 * The naive shape for duplicate detection is to compare every pair, which is
 * quadratic in the number of declarations and turns a cheap check into a
 * budget item. This inverts it: the existing declarations become the *options*
 * of a single Choice, so one call per candidate answers "which of these, if
 * any, is this the same fact as" — linear, and it arrives with a probability
 * for every candidate rather than a yes/no per pair.
 *
 * `existing` is capped by the caller at the API's 255-option ceiling.
 */
export function duplicateOfQuestion(candidate, existing) {
  const criteria = {
    none_of_these:
      "This declares something genuinely new — no option above describes the " +
      "same underlying fact.",
  };
  for (const [name, description] of Object.entries(existing)) {
    criteria[name] = description;
  }

  return {
    type: "choice",
    instructions: {
      question:
        "The declaration in `candidate` is being added. Which of the options " +
        "describes the SAME underlying fact — the same real-world quantity, " +
        "setting, or string — even if it is named differently or expressed in " +
        "different units? Answer `none_of_these` unless it is genuinely the " +
        "same fact restated.",
      candidate,
    },
    criteria,
  };
}

/**
 * Questions asked about a recorded decision whose surrounding facts have moved.
 *
 * Decisions in this platform are append-only with a written rationale, which
 * means the rationale can quietly stop being true: nothing links a decision to
 * the facts it rests on, so nothing notices when one of them changes. Code
 * cannot notice either — it would have to understand the reasoning.
 */
export const DECISION_QUESTIONS = {
  rationale_invalidated: {
    type: "noul",
    instructions:
      "The recorded decision is in `decision`, and the facts as they stand " +
      "now are in `current_facts`. Does the decision's stated rationale rest " +
      "on something that is no longer true?",
    criteria: {
      true:
        "The reasoning depends on a fact, constraint, or assumption that has " +
        "since changed — the decision may deserve a superseding one.",
      false:
        "The rationale still holds against the current facts, or nothing it " +
        "relies on has moved.",
    },
  },
};

/**
 * Questions asked about a product's thesis.
 *
 * A thesis is the one claim a product is built to test, declared in
 * `thesis.toml` (`docs/specs/2026-09-21-the-thesis-is-a-fact.md`). Everything
 * about its *structure* is already checked by code — `fid thesis` reports which
 * fields are empty, and the parser rejects an entry that supersedes one that
 * does not exist. None of that is asked here.
 *
 * What is asked is the part no rule reaches: whether the sentence is actually a
 * thesis. A claim nobody could disagree with is a slogan, and no amount of
 * schema tells you which one you wrote.
 *
 * This is advice about the most personal thing in the repository, so the
 * thresholds matter more than usual. A tool that tells you your thesis is weak
 * every time you run it is a tool you stop running, and then the one time it
 * was right you are not listening.
 */
export const THESIS_QUESTIONS = {
  not_falsifiable: {
    type: "noul",
    instructions:
      "A thesis is a claim some future observation could prove wrong. The " +
      "claim is in `claim`. Is it unfalsifiable — is there no observation, " +
      "even in principle, that would show it to be false?",
    criteria: {
      true:
        "Nothing could count as evidence against it: it is a definition, a " +
        "value statement, an aspiration, or so hedged that any outcome is " +
        "consistent with it.",
      false:
        "A concrete observation would contradict it, whether or not the " +
        "claim states what that observation is.",
    },
  },

  nobody_disagrees: {
    type: "noul",
    instructions:
      "The claim is in `claim`; `for_whom` and `problem` give its context " +
      "where they are declared. Would essentially every informed person in " +
      "this field already agree with the claim as stated?",
    criteria: {
      true:
        "It is a consensus position, a truism, or a statement of good " +
        "intentions no one would argue against — so believing it commits you " +
        "to nothing and it cannot guide a decision.",
      false:
        "A reasonable, informed person could hold the opposite, or could " +
        "argue the emphasis is wrong.",
    },
  },

  describes_instead_of_claiming: {
    type: "noul",
    instructions:
      "A thesis asserts something contestable about the world; a description " +
      "says what the product is or does. Does `claim` merely describe the " +
      "product rather than assert a position?",
    criteria: {
      true:
        "It reads as a summary of features, technology, or scope — what is " +
        "being built rather than what is being claimed about the world.",
      false:
        "It takes a position: something is true, or should be, and the " +
        "product is the bet on it.",
    },
  },

  falsifier_does_not_falsify: {
    type: "noul",
    instructions:
      "`claim` states the thesis and `falsified_by` states what its author " +
      "believes would disprove it. Would observing exactly what " +
      "`falsified_by` describes leave the claim standing?",
    criteria: {
      true:
        "The stated falsifier tests something adjacent, weaker, or " +
        "unrelated — the claim could survive it intact, so it is not the " +
        "test it is presented as.",
      false:
        "Observing it would genuinely force the author to abandon or revise " +
        "the claim.",
    },
  },

  dissent_is_a_strawman: {
    type: "noul",
    instructions:
      "`disagrees` names the person who holds the opposite of `claim` and " +
      "what they believe. Is that opposing position a strawman — too weak, " +
      "too foolish, or too easily dismissed to be the real objection?",
    criteria: {
      true:
        "Nobody serious holds the stated position, or it is phrased so that " +
        "holding it looks obviously wrong, which means the real objection " +
        "has not been faced.",
      false:
        "It is a position a competent, well-intentioned person actually " +
        "holds for reasons that make sense from where they stand.",
    },
  },

  copy_contradicts_the_claim: {
    type: "noul",
    instructions:
      "`claim` is the declared thesis. `restatements` holds hand-written " +
      "product copy — README openings, meta descriptions, landing text — " +
      "found mechanically, each labelled with the file it came from. Does at " +
      "least one of them sell something materially different from the claim?",
    criteria: {
      true:
        "A restatement emphasises a different benefit, drops the part of the " +
        "claim that makes it contestable, or promises something the claim " +
        "does not support — so a reader meets two different products.",
      false:
        "They are consistent with the claim: shorter or differently worded, " +
        "but selling the same thing.",
    },
  },

  overclaims_against_evidence: {
    type: "noul",
    instructions:
      "`evidence_not_yet` lists what this product has explicitly NOT shown. " +
      "`restatements` holds its hand-written public copy. Does any of that " +
      "copy assert, or let a reader assume, something `evidence_not_yet` " +
      "says has not happened?",
    criteria: {
      true:
        "Public copy states or strongly implies a capability, deployment, " +
        "result, or user base that the product's own evidence says does not " +
        "exist yet.",
      false:
        "The copy stays within what has actually been shown, or marks the " +
        "rest as intended rather than done.",
    },
  },
};

/**
 * How much each thesis finding matters, and what to print when it fires.
 *
 * `overclaims_against_evidence` outranks everything, including the questions
 * about whether the thesis is any good. A weak thesis costs you focus; public
 * copy claiming a wildfire-detection deployment that does not exist costs
 * someone's trust, and it is the failure fon's MISSION.md already has to guard
 * against in prose.
 */
export const THESIS_WEIGHTS = {
  overclaims_against_evidence: {
    weight: 1.0,
    title: "public copy may claim more than the evidence supports",
    advice:
      "Say the unproven part plainly in the copy, or move it out of " +
      "`evidence.not_yet` because it is now true. Nothing else here matters " +
      "as much.",
  },
  not_falsifiable: {
    weight: 0.95,
    title: "the claim may not be falsifiable",
    advice:
      "Write what you would have to observe to admit it is wrong. If nothing " +
      "would, it is a value, not a thesis — and it cannot settle an argument.",
  },
  nobody_disagrees: {
    weight: 0.9,
    title: "the claim may be one nobody would argue with",
    advice:
      "Find the sentence a competent person in this field would push back " +
      "on. A claim everyone shares decides nothing.",
  },
  copy_contradicts_the_claim: {
    weight: 0.85,
    title: "product copy may sell something other than the claim",
    advice:
      "Either the copy drifted or the thesis has moved on without being " +
      "updated. Decide which, and if it is the thesis, supersede it.",
  },
  describes_instead_of_claiming: {
    weight: 0.8,
    title: "the claim may describe the product rather than assert anything",
    advice:
      "Rewrite it as something that could be true or false about the world, " +
      "not as what you are building.",
  },
  falsifier_does_not_falsify: {
    weight: 0.7,
    title: "the stated falsifier may not actually test the claim",
    advice:
      "Check that observing it would really make you abandon the claim. If " +
      "not, it is reassurance rather than a test.",
  },
  dissent_is_a_strawman: {
    weight: 0.6,
    title: "the opposing position may be a strawman",
    advice:
      "State the objection the way someone who holds it would state it. The " +
      "real one is the one you have to answer.",
  },
};

/**
 * Turn raw answers into weighted, ranked findings.
 *
 * Two thresholds rather than one, because a probability is not a verdict.
 * Above `report`, the model is confident enough that a reader should act;
 * between `mention` and `report` it is worth a glance and is labelled as such.
 * Below `mention`, silence — an advisory tool that reports everything trains
 * people to ignore it, which costs more than the check was worth.
 *
 * Choice findings additionally require `confidence`, since a Choice's top
 * option can win a near-uniform distribution and mean nothing.
 *
 * `weights` selects which question set is being weighed — `DIFF_WEIGHTS` for a
 * working diff, `THESIS_WEIGHTS` for a thesis. A key with no entry in the
 * chosen set is skipped, which is what keeps a stray answer from one set out of
 * the other's findings.
 */
export function weigh(
  answers,
  { mention = 0.55, report = 0.8, choiceConfidence = 0.6, weights = DIFF_WEIGHTS } = {},
) {
  const findings = [];

  for (const [key, answer] of Object.entries(answers)) {
    const spec = weights[key];
    if (!spec) continue;

    if (answer.type === "noul") {
      if (answer.noul < mention) continue;
      findings.push({
        key,
        title: spec.title,
        advice: spec.advice,
        probability: answer.noul,
        firm: answer.noul >= report,
        score: answer.noul * spec.weight,
      });
      continue;
    }

    if (answer.type === "choice") {
      // The escape option is the clean answer for this question, not a finding.
      if (answer.choice === "already_at_the_right_layer" || answer.choice === "none_of_these") {
        continue;
      }
      const p = answer.probabilities?.[answer.choice] ?? 0;
      if (p < mention || (answer.confidence ?? 0) < choiceConfidence) continue;
      findings.push({
        key,
        title: `${spec.title} (${answer.choice})`,
        advice: spec.advice,
        probability: p,
        firm: p >= report,
        score: p * spec.weight,
      });
    }
  }

  return findings.sort((a, b) => b.score - a.score);
}
