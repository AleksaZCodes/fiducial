# The platform names two kinds of model, and `Decision` is not available to either

**Date:** 2026-09-20
**Status:** accepted
**Supersedes:** nothing

---

## Context

Until now this platform reached exactly one kind of model, through one contract
called `Ai` (`[adapters] ai`), and the word "AI" unambiguously meant it: send
turns, get text or tool calls back.

That stopped being true when a second, genuinely different kind arrived. A
*System One* model — TypeSafe's Jev is the first — takes a state and a map of
named questions and returns **typed answers with calibrated probability
distributions**. It generates no text at all; OpenRouter marks these models
`output_modalities: ["decisions"]` and `has_text_output: false`, which is why
they do not appear in a default `/api/v1/models` listing and have their own
endpoint at `POST /api/alpha/decisions`.

Two collisions appeared immediately, and both were live in the working tree
before this was written down:

1. **`ai` had come to mean two things.** It was the name of the generative
   contract *and* it was being used as the umbrella for anything model-shaped:
   the first version of key management shipped as `fid ai key set`, storing a
   key that in fact serves **both** contracts. A key that serves two contracts
   filed under the name of one of them is a small lie a reader has to unlearn.

2. **`decisions` was the obvious vendor-neutral name, and it is poisoned.**
   OpenRouter's endpoint is `/api/alpha/decisions`; the modality is
   `"decisions"`; the industry is converging on the word. But **`Decision` is
   one of this platform's five primitives** (`ARCHITECTURE.md` §2): a declared
   human judgment with rationale and date, appended and never edited,
   permanent, auditable, committed to `docs/specs/` — this file is one.

   A model's answer is the opposite of that in every respect: inferred not
   declared, probabilistic not certain, ephemeral not recorded, advisory not
   binding. Naming the contract `decisions` would have put two exact opposites
   behind one word in one repository.

## Decision

**Three names, each saying what the thing is.**

| Concept | Name | What it is |
|---|---|---|
| Generative model contract | `Ai` / `[adapters] ai` | Turns in, text or tool calls out. Unchanged. |
| Decision model contract | `SystemOne` / `[adapters] systemOne` | State plus typed questions in, typed answers with probabilities out. |
| The advisory feature built on it | `fid advise` | Where a key is set, and where asking happens. |

`Decision` stays reserved for the primitive, exclusively.

`SystemOne` is kept as the contract name because it names a **category, not a
vendor**. "System 1" is Kahneman's — fast, automatic, intuitive cognition — and
predates TypeSafe by a decade and a half. It happens to describe this class of
model exactly, and it carries a useful corollary: the `Ai` contract is the
System 2 of this platform, slow and deliberative. A model that is not Jev still
fits the name, which is what principle 6 requires of a contract.

**Key management hangs off `fid advise`, not off `ai`.** `fid advise key set`
names what the key unlocks rather than which contract it happens to
authenticate. It is honest in a second direction too: this key is for
developer-machine advisory tooling only — a product's runtime secrets go to
`wrangler secret put` and never to the credentials file.

## Why not the alternatives

**Rename `ai` → `generative` or `llm`.** Honest, and it would have made `ai` a
clean umbrella. Rejected because `[adapters] ai` is a committed key in every
product that has one, and `config.rs` already declines exactly this trade for
`errors` vs `Diagnostics` — "renaming a `fiducial.toml` key is a breaking change
for no benefit." Here there *was* a benefit, but a smaller one than the break:
moving the ambiguity out of the *command* surface removed it from everywhere a
person actually types, at zero cost to existing products.

**Name the new contract `decisions`.** Vendor-neutral, matches the emerging
industry term, single lowercase word matching the dominant key style. Rejected
on the collision above, which is not a near-miss but an inversion of meaning.

**Name it `inference`.** Vendor-neutral and accurate, but it describes the `Ai`
contract equally well — every LLM call is inference — so it distinguishes
nothing.

**Fold it into `Ai` as a method.** Rejected in the contract's own doc comment:
the two shapes have no honest intersection, and coercing typed decisions through
a chat-completions shape is the precise mismatch a System One model exists to
remove.

## Consequences

- `ai` now means generative, narrowly and only. Any future umbrella needs a new
  word; it may not reuse this one.
- `Decision` is load-bearing vocabulary. A reviewer should reject a variable,
  type, or config key that uses it for a model output.
- One OpenRouter key serves both contracts, which is why `openrouter` is the
  default vendor for `systemOne`: one secret to set and rotate, not two.
- `systemOne` is the second camelCase multiword adapter key, after
  `botProtection`. Files stay kebab-case (`system-one.ts`), matching
  `bot-protection.ts`.
- This costs a paragraph of explanation whenever someone meets the platform,
  because "AI" in common speech covers both kinds and here it does not.

## What this deliberately does not do

- **Does not rename `[adapters] ai`.** Existing products keep working untouched.
- **Does not put a model anywhere near derivation.** Orthogonal to naming but
  stated here because the two get conflated: AI in this platform advises and
  never derives. `fid derive` is byte-identical with and without a key, and
  `crates/fiducial-cli/tests/determinism.rs` proves it by deriving a product
  twice and hashing both trees. A gate whose verdict varies is not a gate.
- **Does not claim `SystemOne` is a neutral industry standard.** It is a
  category name that currently has one vendor in it. If the category settles on
  a different word, that is a superseding decision — write a new file.

---

<!--
Decisions are append-only (principle 1b). When this is superseded, write a NEW
dated file that says so and set Status above. Never edit the reasoning — being
able to see what was believed at the time is most of the value.
-->
