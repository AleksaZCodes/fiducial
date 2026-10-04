# sensor-stick — Agent Context

This file is read by Claude Code (and any other agent) at session start.
It records what is true about this product so that agents do not have to
re-derive it.

## Platform

Built on [Fiducial](https://github.com/AleksaZCodes/fiducial) 0.9.3.

- `fid dash` — one read-only view of this product's state; start here
- `fid doctor` — check for drift before starting work
- `fid derive` — run pipelines; `--check` fails CI on stale artifacts
- `fid graph` — the facts → pipelines → artifacts DAG
- `fid upgrade` — pull upstream template and package updates

`fid dash --json` is the machine-readable form of the same facts. Prefer it over
re-deriving product state by reading files: it already reports what is stale,
which pipelines exist, what decisions have been recorded, and whether CI guards
artifact freshness.

## Principles

These are the platform's, not this product's. They are **generated** from the
platform's `MISSION.md` when this file is scaffolded — there is exactly one place
they are authored, and this is not it. Do not edit them here; the next
`fid upgrade` regenerates this section.

They are restated in your repository rather than linked because an agent working
here should not need the platform checked out to know the rules.

**1 · One declaration, many derivations.**
A fact used by two domains is declared in one place and generated into both. If you are typing
the same value into a second file, stop: that value belongs upstream. Hand-writing a derived
artifact is an error, not a shortcut.

A physical fact is never a bare number. It carries its **unit** and its **tolerance**, because
the question that actually matters — will it fit, will it last, may I claim it — is answered by
stacking tolerances across domains, not by comparing nominal values.

**1b · A judgment that cannot be derived is still declared.**
Decisions, and the reasoning behind them, are written down and versioned like any other input.
Undocumented judgment is re-litigated, by you in six months and by every agent that follows.

**1c · A user-visible string is a fact.**
It is declared once, and every locale is a derivation that must exist. A missing
translation is a **missing artifact, not a fallback** — and like any stale
artifact, it fails the build rather than reaching a reader.

Localization is not a later pass. A system that can be built monolingual will be
built monolingual, and the strings missed on the way are invisible: they render
as plausible text in the wrong language, and only a human reading that language
ever finds them. So the default is **localized by construction** — monolingual is
a state a product passes through before its first commit, not a state it ships.

The same reasoning covers every other word a person reads: legal text, email
copy, error messages. Copy is content, content is declared, and declared things
are derived into every form they are needed in.

**2 · Push behavior down to the layer with the longest reach.**
Logic placed in a `no_std` Rust core runs in a browser, on a desktop, at the edge, and on a
microcontroller. Logic placed in a UI framework survives until you change frameworks. Prefer the
former; it costs the same to write and lasts an order of magnitude longer.

**3 · No abstraction without an escape hatch.**
Work at the highest level that suffices, and be able to drop to a lower one without abandoning
the system. Every generated artifact is human-readable. Every high-level declaration has a
documented override. An abstraction you cannot escape is a trap, and the first hard problem will
find its edge.

**4 · Everything explicit, in git, legible to a machine.**
State that lives only in someone's head, a vendor dashboard, or a chat log cannot be derived
from, reviewed, reverted, or reasoned about by an agent. Structured files beat prose; prose beats
memory. This is what makes autonomous agent work safe rather than hopeful.

**5 · Cost is a design variable, declared before the design — not measured after it.**
A cost ceiling is a **fact**, set first, with the assertion failing until the design meets it.
This is the inversion that matters: you do not discover what something costs, you are constrained
by what it must cost. Manu Prakash's frugal science is the proof it works — a 97¢ paper
microscope, a 20¢ paper centrifuge replacing a $1,000 machine, a $100 electron microscope
replacing a $60,000 one. None of those is a cheaper version of the expensive thing; each is a
**fundamentally different design that only a hard ceiling would have forced anyone to find.**
Constraints are not a tax on invention. They are the mechanism of it.

Every cost is therefore derived and asserted, never estimated: bill of materials, material mass,
machine time, cloud spend, **and the cost of the agents doing the work.** Prefer free and open
tooling; prefer the cheapest process that meets the requirement.

**5b · Spend upfront where it compounds down.**
Effort invested in the system is repaid by every product built on it, so the correct time to pay
is early. The purpose is not thrift for its own sake — it is to drive the marginal cost of the
next product toward zero, leaving the budget and the attention for invention rather than
rebuilding.

**5c · Order work by cost of delay, not by value.**
An item belongs early when *waiting makes it more expensive* — which is not the
same as wanting it most. Three kinds, and the order follows from the kind:

- **Debt-accruing** — the cost grows with every cycle you wait, usually because
  work done meanwhile has to be migrated later. These go first, even when they
  are not the most wanted.
- **Multiplying** — they make every later piece of work cheaper. These go second.
- **Terminal** — they cost the same whenever you do them. These go last, ordered
  by product value.

Value and urgency are different axes, and ranking by value alone reliably
schedules the compounding work last, where it costs the most. 5b says spend
early where it compounds; this says how to find those places.

**6 · Commit to contracts, not to tools.**
Be opinionated about the interface between domains and permissive about what satisfies it. One
contract per domain, fixed; the tool behind it, swappable. This is how a system stays sharply
opinionated — which is where the efficiency comes from — without ever being locked in. A tool that
cannot meet the contract is one this system declines to adopt; a tool that can is a one-line change.

**7 · The system compounds.**
A bug fixed once propagates to every product that shares the code. A learning is written where
it will be read again. A pattern that worked becomes a template. Nothing correct is solved twice
— and a fix that cannot propagate is only half-finished.

## Product layout

```
sensor-stick/
├── fiducial.toml   — product config, capabilities, guard rules
├── fiducial.lock   — template version tracking (do not edit by hand)
├── MISSION.md      — what this product is for
└── AGENTS.md       — this file
```

## Agents

Two specialised agents are pre-configured in `.claude/agents/`. Both use Opus.

| Slash command | When to use |
|---|---|
| `fiducial-design` | Architecture decisions, brainstorming, new capability design |
| `fiducial-review` | Review the current diff before committing or opening a PR |
| `/fiducial:platform` | Load full platform context (via Fiducial plugin) |

The default session model (Sonnet) handles implementation. Switch to
`fiducial-design` or `fiducial-review` when the task is judgment, not code.

The names are namespaced on purpose: a subagent's filename is its identity, so
an agent called `design` collides with any other `design` agent you have, and
the loser is silently unavailable.

Review happens when a person asks for it — `fiducial-review`, or `/code-review`
before opening a PR. A scaffolded product no longer ships a CI job that reviews
every pull request automatically: it needed `ANTHROPIC_API_KEY` in the
repository's secrets to do anything, it commented on every PR whether or not
the diff warranted it, and a review nobody asked for is a review nobody reads.

## Do not script what a tool already does

Editing a file by piping it through a Python or Node heredoc is slower and less
safe than the harness's own file tools, and the difference is not stylistic:

**A scripted replacement fails silently.** `s.replace(old, new)` against a
pattern that is not there returns the string unchanged and writes it back, and
the script reports success. The edit did not happen, and you find out two build
cycles later — or not at all. `Edit` validates the match, refuses an ambiguous
one, and errors loudly when the text is not found. That is the whole reason to
prefer it.

| Doing | Use | Not |
|---|---|---|
| changing a file's contents | `Edit` | `python3 - <<'PY'`, `node -e` |
| writing a new file | `Write` | `cat > f <<'EOF'` for anything structured |
| reading a file | `Read` | `cat`, `sed -n` |
| finding text across files | `Grep` | `grep -r` piped through three filters |
| finding files by name | `Glob` | `find` with `-name` and `-not -path` |

Shell is still right for what shell is for — the build, the tests, `git`, `fid`,
a one-off `wc -l`. Reach for a scripting language when the task is genuinely a
program: parsing JSON to answer a question, arithmetic over a data file,
generating a fixture. Not to perform an edit.

## Capability instructions

Every installed capability ships instructions for using it. They are **ordinary
Markdown at a vendor-neutral path**, so any agent can read them:

```
.fiducial/skills/<capability-id>.md
```

The installed capability ids are in `fiducial.toml` under `[capabilities]
enabled`. Read the file for each one before working on that part of the product.

Claude Code additionally gets a short pointer at `.claude/skills/<id>.md` so its
automatic discovery works. That file contains **no instructions of its own** —
it points here. The content exists once.

These files are platform-owned and rewritten by `fid upgrade`. Do not edit them.

## Guard rules

The `PreToolUse` guard (`fid guard-check`) is contributed by the Fiducial Claude Code
plugin (`.claude/settings.json`). It enforces the rules listed in `fiducial.toml [guard]`.
Rules fire only when a forbidden command is in **command position** — not when it
appears inside a quoted string or as an argument.

## When you hit a wall

Fix it where it lives, not where you hit it.

- **A platform defect** — a `fid` bug, a wrong template, a capability that is
  missing or cannot express what this product needs: open an issue or a pull
  request on [Fiducial](https://github.com/AleksaZCodes/fiducial) with the fix.
  If this product cannot wait, carry the fix here as a **recorded** fork —
  `fid rebaseline` for a platform-owned file, or a patch under `patches/` named
  after the upstream issue — and delete it when upstream ships. Never an
  unrecorded hand-edit: it is drift nobody can see.
- **A tool that cannot do it** — prefer existing open-source software behind a
  contract (principle 6); build only what nobody has; build around it only when
  both cost more than the problem.
- **Change the system, not just the instance.** A change to how something works
  updates the declaration, the docs and the agent instructions that describe it
  in the same change. Documentation that lags is a stale artifact.
- **Teach the next agent only what is fundamental.** Add a line to this file when
  a lesson would have prevented a *class* of mistakes, not for a one-off.
  Instructions that grow without bound stop being read.

## What not to do

- Do not hand-edit files listed in `fiducial.lock` (they are template-tracked).
- Do not commit directly to the default branch (`no-direct-main-push` rule).
- Do not hand-write derived artifacts — change the upstream declaration and
  re-run `fid derive`.
