# MISSION

**This file states what the system is for.** When a decision is genuinely ambiguous, it is the
tiebreaker.

It is *not* copied verbatim into products — a product writes its own `MISSION.md` saying what
that product is for. What every product inherits is the **principles**, restated in its
scaffolded `AGENTS.md` so that agents working there have them without needing this repository.

(This paragraph used to claim the file was copied into every repository. It never was. The claim
survived because nothing checked it — the same drift the principles below exist to delete.)

---

## The mission

Build **cross-domain products** — electronics, firmware, protocols, web, desktop, mobile,
mechanical, simulation, content — at a quality and pace that normally needs a team, run by one
person working with agents.

Not by working harder at each domain. By removing the work that exists only because the domains
don't talk to each other.

---

## The thesis

> **Declare each fact once. Derive every artifact from it.**

In a cross-domain product the same fact is normally written down many times — a pin assignment
in the schematic, the firmware, the test rig and the docs; a protocol message in the firmware
and again in the web client; a board dimension in the PCB tool, the enclosure model and the
marketing render. Every duplicate is a place where reality drifts from itself, and drift is
discovered late, in the field, expensively.

So: one declaration, many derivations. The declaration is typed, machine-readable, and lives in
git. Everything downstream is generated and never hand-edited.

**The consumer of this rule** is anyone — human or agent — who changes a fact and needs every
consequence to follow automatically instead of by memory.

---

## Seven principles

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

---

## What good looks like

- A new product reaches a running local environment in **minutes**, at the engineering standard
  the last product *ended* at, not at zero.
- Changing a protocol message is **one edit**, and every endpoint that no longer matches **fails
  to compile**.
- Changing a board dimension updates the enclosure, the render, and the spec sheet **without
  anyone remembering to**.
- A fix made in shared code reaches every other product with **one command**.
- Time goes to judgment — what to build, how it should feel, whether it is correct — and not to
  administration, reconciliation, or re-typing facts the system already knows.

---

## Anti-goals

Named so they are boundaries, not oversights. Each has cost someone a year.

- **We do not rebuild what already works.** Not a CAD kernel, not an EDA suite, not a browser
  engine, not a cloud. We stand on existing tools and own the *integration* between them — that
  integration is where the value is and where nobody else is working.
- **The platform must never become the project.** Nothing is added speculatively. A capability
  enters the platform when a real product needs it, and is generalized when a **second** one
  does. Shipping products is the point; the platform is the residue of having shipped them.
- **No component owns data it did not declare.** Dashboards, docs sites, and status pages are
  *views* over the repository. The moment a view holds its own copy, it must be kept in sync, and
  keeping things in sync is the work this system exists to delete.
- **No silent state.** Nothing important lives outside version control.
- **No abstraction without an exit.** See principle 3.

---

## The bar

One person, working with agents, ships a product that is **indistinguishable in quality from a
funded team's** — tested, documented, observable, maintainable — and starts the next one from a
stronger position than the last.

Every rule in this system either serves that, or is removed for being tax.
