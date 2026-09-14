---
description: Translate an existing codebase into reusable assets — extract its business logic, art, UI/UX and principles, and generalize them into this product without overriding it
---

You are extracting reusable work from a **donor codebase** into the product you
are currently in.

The user has built something before. It contains work that should not be built
twice: business rules that took iterations to get right, a theme that took a week
to tune, components, copy, and conventions arrived at the hard way. Your job is
to get that value into this product **generalized**, not pasted.

---

## Before anything else

```sh
fid harvest <path-to-donor> --name <slug>
```

If `harvest/<slug>/` already exists, the survey has been run — do not re-run it
unless the donor has changed.

Then read, in this order:

1. `harvest/<slug>/SURVEY.md` — the survey, ordered by value per unit of risk
2. `harvest/<slug>/harvest.toml` — the machine-readable inventory
3. `MISSION.md` — the tiebreaker for every judgment below
4. `fiducial.toml` — what this product already has

The staged copies live in `harvest/<slug>/assets/`. **Read them there.** They are
reference material, not source.

---

## The rule that governs everything here

> **`harvest/` is a staging area. It is never the product.**

You are not importing a codebase. You are reading one and deciding what deserves
to exist in this product, in this product's shape.

Three things follow, and they are not negotiable:

- **Never overwrite existing product source with donor source.** If a donor file
  and a product file do the same job, the product's version wins by default. Say
  what the donor does better and let the user decide.
- **Never paste a donor file into the product unchanged.** If it is worth
  keeping, it is worth renaming, retyping and stripping of the donor's nouns. An
  unchanged paste is a second declaration of somebody else's assumptions.
- **Never wire in anything the user has not agreed to.** Propose, then do.

If the user says "just copy it in", tell them once what that costs — a copy that
now has to be kept in sync with a repository nobody will open again — and then do
what they asked.

---

## The extraction, kind by kind

The survey orders the work. Follow that order: it goes from lowest risk and
highest certainty to highest.

### 1 · theme — do this first

Almost always worth lifting, almost never risky. Tokens are declarative, have no
dependencies, and carry the thing that took longest to get right.

**What to do:** find the donor's colors, spacing, typography, radii and shadows.
Reduce them to a token set. Land them in `@fiducial/tokens` if this product uses
it, or in the product's token file otherwise.

**What to watch for:**
- A donor usually has *both* a token system and a pile of one-off values that
  drifted from it. Lift the system; report the drift rather than importing it.
- Hardcoded hex values scattered through components are a finding, not tokens.
- If the donor's palette is close to this product's, do **not** replace this
  product's — report the difference and ask.

### 2 · principle — cheap, and high leverage

Decision records, conventions, READMEs, architecture notes.

**What to do:** read them for *judgment* — the reasoning behind choices, not the
choices themselves. Anything that is still true and still relevant belongs in
this product's `docs/specs/` as a **new, date-stamped decision** that credits
where it came from.

Per principle 1b: a judgment that cannot be derived is still declared. An
undocumented decision gets re-litigated, by the user in six months and by every
agent that follows.

**What to watch for:** most donor prose is stale status reporting. Keep the
reasoning, discard the state.

### 3 · contract — schemas, shared types, migrations

**What to do:** these are the best candidates in any donor for an actual
*improvement*. A donor typically has two hand-written copies of a shape — one in
the database, one in the client. This platform's answer is **one declaration and
two derivations**.

So do not port both copies. Port the shape once, into the place it should be
declared, and let the other side be generated.

### 4 · logic — the highest value and the highest care

**What to do:**

1. **Separate the rule from its plumbing.** The survey flags files as
   "logic mixed with effects". Those contain a pure rule wrapped in `fetch`,
   `process.env` and database calls. The rule is what you want.
2. **Ask how far down it can go.** Per principle 2, logic in a `no_std` Rust core
   runs in a browser, on a desktop, at the edge, and on a microcontroller. Logic
   in a framework survives until the framework changes. If the rule is genuinely
   domain logic and not presentation, propose L0 Rust.
3. **Port the tests, or port nothing.** The value in mature business logic is the
   edge cases someone already found — and those live in the tests, not the happy
   path. If the donor has no tests for a rule, write characterization tests
   against the donor's behaviour *before* you rewrite it.

**What to watch for:**
- Logic that encodes one product's business model is not reusable, however clean
  it looks. Say so and move on.
- A 1900-line file is not one rule. Find the three that matter.

### 5 · ui — generalize the shape, drop the nouns

**What to do:** identify what is *structural* (a card with a header, a media
slot and an action row) versus what is *this product* (the words, the entities,
the routes). Keep the structure. Drop the nouns.

For a landing page specifically — which is the most common thing worth reusing —
what transfers is: the section order, the spacing rhythm, the type scale, the way
the hero resolves, the density of the proof sections. What does not transfer is
the copy.

**What to watch for:**
- A component wired to the donor's data layer is not a component, it is a page.
  Extract the presentational part.
- Land reusable components in a registry package (`@fiducial/ui-react`,
  `@fiducial/ui-svelte`) rather than copying them product to product — otherwise
  you are rebuilding the duplication you just removed.
- Accessibility defects transfer perfectly. Check before adopting.

### 6 · ops — read every line

Scripts, workflows, deployment config. Often reusable nearly verbatim, which is
exactly why they are dangerous.

**What to watch for, before anything else:**
- **Secrets, account IDs, project refs, tokens, bucket names, connection
  strings.** Assume they are present until you have checked. Never copy one into
  this product, and never print one into the conversation.
- Workflows referencing repositories, environments or org names that do not exist
  here.

### 7 · art — provenance first

**What to do:** check licensing before anything else. Fonts especially: a webfont
licensed for one domain is not licensed for this product.

Only then consider whether it is worth having. Most donor art is product-specific
and does not transfer.

---

## How to report

Work through the kinds in order. For each one, before you change anything,
present:

| | |
|---|---|
| **What is there** | what you actually found, with file paths |
| **What is worth lifting** | and what specifically it would become here |
| **What is not** | and why — this is as valuable as the first column |
| **What it costs** | new dependencies, new files, anything it would replace |

Then wait for a decision before writing to the product's source tree.

Keep a running record in `harvest/<slug>/EXTRACTION.md`: what was lifted, what
was rejected and why, what remains. A rejection with a reason is a decision, and
per principle 1b decisions get written down — otherwise the next person to look
at this donor re-litigates every one of them.

---

## When you are done

1. `fid doctor` — the product is still in order
2. `fid derive --check` — nothing derived went stale
3. Run the product's tests
4. Add a dated decision to `docs/specs/` recording what was harvested from where,
   what was deliberately left, and why
5. Leave `harvest/<slug>/` in place. It is the evidence for those decisions, and
   it is cheap. Add it to `.gitignore` only if the user asks.

---

## The three failure modes

**Importing instead of extracting.** You end up with a second codebase inside
this one, carrying assumptions nobody chose, needing sync with a repository
nobody opens. Signal: files in the product that still use the donor's vocabulary.

**Lifting the artifact instead of the judgment.** Copying a config file rather
than understanding which two settings in it mattered. Signal: you cannot explain
why a value is what it is.

**Being agreeable about quality.** The donor's bugs, dead code and accessibility
defects transfer perfectly well if you are not reading. The user asked for the
good parts. Finding that a well-liked component has a real defect is a useful
result — say so.
