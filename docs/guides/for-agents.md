# For agents

> How to work in a Fiducial product without breaking the property that makes it
> worth using.

Written for AI agents, and useful to humans who want to know what the agents
have been told.

---

## Orient, in four commands

Run these before doing anything. They are cheap, they make no network calls, and
they answer most of what you would otherwise guess at.

```sh
fid dash --json    # the whole product as structured facts
fid doctor         # is it internally consistent?
cat fiducial.toml  # capabilities and guard rules
cat MISSION.md     # the tiebreaker for ambiguous decisions
```

`fid dash --json` exists specifically for you. It emits the same facts the text
view shows, so you do not re-derive them by reading files, and so a future tool
does not reimplement these reads a third time.

Then read `AGENTS.md` in the product root for anything product-specific.

---

## The one rule that matters most

> **Never hand-edit a generated file. Change its declaration and re-derive.**

Generated files are recorded in `fiducial.lock`. A `PreToolUse` guard hook blocks
edits to them, so you will usually be stopped — but being stopped is a poor way
to learn this, and the guard cannot catch every route.

When you want to change a generated artifact, the question is always *"what
declaration produced this?"* Find it with:

```sh
fid graph          # every pipeline: inputs → executor → outputs
```

Then edit the input and run `fid derive`.

**Why this is enforced rather than encouraged:** a hand-edited artifact is
correct exactly until the next `fid derive`, which silently reverts it. The
change appears to work, ships, and disappears. That failure mode is invisible in
review, which is why it is blocked in tooling.

---

## The five that will bite you

**1 · Typing a value a second time.**
If you are about to write a number, a string, or a version that already exists
somewhere in the repository — stop. It belongs in one declared place with the
second use derived from it. This is the platform's entire thesis and the most
common way to violate it is by being helpfully thorough.

**2 · Writing logic in the top layer.**
Before writing a rule in TypeScript, ask whether it belongs in `no_std` Rust. A
rule in the core runs in a browser, on a desktop, at the edge, and on a
microcontroller. A rule in a UI framework survives until the framework changes.
If it is domain logic rather than presentation, propose pushing it down.

**3 · Adding a capability nobody asked for.**
`MISSION.md`: *the platform must never become the project.* A capability enters
when a real product needs it, and is generalized when a **second** one does.
Speculative generality is a defect here, not foresight.

**4 · Editing a past decision.**
`docs/specs/` is append-only. When a decision is superseded you add a new,
date-stamped one that says so. Do not edit the old file — the history is the
record, and the reasoning that was wrong is often the most useful thing in it.

**5 · Reporting green when something was skipped.**
If a check did not run, say it did not run. This repository has already been
bitten by a `typecheck` script that was an `echo` and reported success for two
phases while checking nothing. An honest failure is worth more than a green tick.

---

## Before you say you are done

```sh
fid doctor           # config, lock, template integrity, migrations
fid derive --check   # every artifact still matches its declaration
cargo test --workspace --all-features    # if the product has Rust
pnpm test            # if the product has JS
```

`fid derive --check` is the one people skip. It is also the one that catches the
specific failure this platform exists to prevent.

If you changed behaviour, add a test. If you made a judgment call that a future
reader would re-litigate, add a dated decision to `docs/specs/`.

---

## Conventions worth knowing

**Comments say why, not what.** The code says what. A comment earns its place by
recording a constraint, a rejected alternative, or a non-obvious reason. The
codebase this platform was harvested from was harvestable *because* of this
habit — it was possible to tell a deliberate approximation from a bug.

**Errors name the fix.** An error that says what is wrong but not what to change
is half an error. `ValidationError` names the connector *and* the field.

**Tests assert properties, not implementations.** The mesh tests do not check
triangle counts; they assert the mesh is watertight by directed-edge uniqueness
and positive signed volume, then ray-probe the shipped STL bytes. Write the test
that would catch the bug, not the test that describes the code.

---

## Reusing an existing codebase

If the user wants to bring work over from another repository:

```sh
fid harvest <path> --name <slug>
/fiducial:harvest <slug>
```

`harvest/` is a staging area and never the product. Do not paste donor files into
the source tree. See [Harvesting](./harvesting.md) — and note that the skill's
job includes deciding what is **not** worth lifting.

---

## Available skills

| Skill | For |
|---|---|
| `/fiducial:platform` | Load platform context at session start |
| `/fiducial:harvest` | Extract reusable work from another codebase |

Capability skills (`.claude/skills/*.md`) are installed by `fid add` and
refreshed on every `fid upgrade`. They are platform-owned — do not edit them; the
next upgrade overwrites them, by design.

---

## What good work looks like here

The bar, from `MISSION.md`:

> One person, working with agents, ships a product **indistinguishable in quality
> from a funded team's** — tested, documented, observable, maintainable — and
> starts the next one from a stronger position than the last.

Concretely, for a change you make:

- The fact it depends on is declared once
- Anything downstream is generated, not written
- New behaviour has a test that would fail without it
- A judgment that could not be derived is written down with its reasoning
- Nothing is reported as working that you did not watch work
