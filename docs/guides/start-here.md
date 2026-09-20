# Start here

> What this system actually is, why it is shaped that way, and what changes
> about how you work.

You do not need to know Rust, embedded development, or CAD to read this. It
assumes you have built software before and have opinions about build tools.

---

## The thing that is actually wrong

Here is a bug that costs people weeks, in every project that touches more than
one domain.

You are building a device with a companion app. The board has a USB-C connector.
That fact — *there is a USB-C connector, 20 mm from the left edge of the south
side* — gets written down:

1. In the schematic, as a component placement.
2. In the firmware, as a pin configuration.
3. In the enclosure model, as a rectangular hole.
4. In the test rig, as a fixture position.
5. In the assembly instructions, as a photo.
6. On the marketing page, as a render.

Six copies of one fact. Now move the connector 3 mm.

You will remember to update the schematic, because that is what you are working
in. You will probably remember the enclosure. You will not remember the test
rig. Three months later, 500 units arrive from the fab and the enclosure does not
close, and the reason is a number in a file nobody opened.

**Nothing here is a hard problem.** Every individual step is easy. The cost is
entirely in the *reconciliation* — in remembering which five other places depend
on the thing you just changed. That reconciliation is most of the work in
cross-domain products, and it is the work this system deletes.

## The move

> **Declare each fact once. Derive every artifact from it.**

You write the connector down **once**, in a typed file that lives in git:

```json
{
  "id": "J1",
  "type": "usb-c",
  "mount": { "side": "south", "offset_mm": 20.0 }
}
```

And then the enclosure, the render, the TypeScript types and the spec sheet are
all **generated from it**. Not "kept in sync with it" — *generated*. There is no
second copy to update, because there is no second copy.

Move the connector 3 mm and every downstream artifact moves, because they were
never independent in the first place.

### The part that makes it real

Any build system can generate files. The thing that makes this trustworthy is
that **a stale artifact fails the build**:

```sh
fid derive --check
```

Every generated file's hash is recorded when it is generated. `--check` re-hashes
them and exits non-zero if any no longer matches its declaration. That runs in
CI. So the guarantee is not "we generate things" — it is *"an artifact that has
drifted from its declaration cannot reach main."*

That is the whole system, in one command. Everything else is machinery for
making more kinds of artifact derivable.

---

## Why this feels different from a build system

A build system compiles source into output. This does that, but the interesting
part is the direction it pushes you.

### You start writing down facts instead of writing code

The board width is not a number in three files. It is a **declaration**, and the
enclosure generator *reads* it. When you want a different case, you do not edit
the case; you edit the fact the case is derived from.

This feels strange for about a day and then feels obviously correct.

### A number carries its tolerance

A physical fact is never bare. `3.3 V ±5%` is a different thing from `3.3 V`, and
the question you actually care about — *will this part work* — is answered by
stacking tolerances, not comparing nominal values:

```rust
let rail = Quantity::<Voltage>::with_pct(3.3, 5.0);   // 3.135 .. 3.465
let part = Quantity::<Voltage>::exact(3.2);           // needs at least 3.2

rail.assert_ge(&part)  // ← FAILS. 3.135 < 3.2
```

Comparing `3.3 > 3.2` passes. It is also wrong, and you find out in the field.

### Logic goes as low as it can

A rule written in a UI framework survives until you change frameworks. The same
rule written in `no_std` Rust runs in a browser, on a desktop, at the edge, and
on a microcontroller — and costs the same to write.

So the default is: push it down. The platform's core compiles for four targets on
every commit, which means a target breaks the day it breaks rather than the day
you need it.

### Decisions are written down like data

Some things cannot be derived. *We chose LoRa over BLE because the site is 2 km
across and the units are battery-fed.* That is judgment, and judgment that is not
recorded gets re-litigated — by you in six months, and by every agent that reads
the repository afterwards.

So decisions go in `docs/specs/`, date-stamped, **append-only**. When a decision
is superseded you add a new one; you do not edit the old one. The history is the
record.

---

## What you actually do

```sh
cargo install fiducial-cli
fid new my-product
cd my-product
fid dash
```

`fid dash` is the one command worth learning first. It shows you the whole
product in one view — roadmap, decisions, CI, pipelines, and whether anything
generated has gone stale:

<!-- capture: fid-dash.txt -->

```text
$ fid dash

Product
  name           demo-product
  version        0.1.0
  root           /home/you/dev/demo-product
  spine          disabled
  capabilities   design, i18n
  guard rules    2

Git
  branch         main
  head           no commits yet
  working tree   16 file(s) with uncommitted changes
  upstream       not tracking a remote branch

Roadmap
  source         ROADMAP.md
  progress       0 done, 0 in progress, 1 to do  (1 tracked)
  next           _Replace this row_ _and say what it costs to defer_

Decisions
  state          none recorded (looked in docs/specs, docs/decisions, docs/adr)

CI
  CI                           on push, pull_request  [checks artifact freshness]
                                 └ fid derive --check
  (declared workflows, not live run status — dash makes no network calls)

Graph
  design (fid-design)
    → apps/web/src/app/tokens.css
  i18n (fid-i18n)
    → src/generated/messages.ts

Freshness
  artifacts      1 fresh, 0 stale, 0 missing, 0 never derived
  templates      unmodified
  everything the lock tracks is current

Localization
  locales        sr, en (default: sr)
  no hardcoded user-visible strings found

Capabilities
  ✓ design-system.md                   declared (design)
  ✓ i18n                               declared (i18n)
  ✓ messages/en.json                   declared (i18n)
  ✓ messages/sr.json                   declared (i18n)

  database       not selected
  storage        not selected
  deploy         not selected
  email          not selected
  newsletter     not selected
  errors         not selected
  botProtection  not selected
  queue          not selected
  ai             not selected
  auth           not selected
  systemOne      not selected
```

<!-- /capture -->

Two properties of that view matter more than what it displays:

**It owns no data.** Every number is recomputed from files already in the
repository. A test asserts the command writes nothing at all. A dashboard that
keeps its own copy has to be kept in sync, and keeping things in sync is the work
we are deleting.

**It makes no network calls.** CI status is read from the workflow files in the
repo, not a live API. A test renders the entire dashboard with every network
proxy pointed at a closed port. So it works on a plane, and it never blocks.

Then: [build something](./first-product.md).

---

## The five rules, and what each one is protecting you from

| Rule | Without it |
|---|---|
| **Declare once, derive the rest** | Six copies of the connector position, and the fab run that finds out |
| **Never hand-edit a generated file** | Your edit is erased on the next build, or worse, it isn't and now the source lies |
| **Derived artifacts are never committed by hand** | CI passes on an artifact that no longer matches its declaration |
| **Decisions are appended, not edited** | You re-argue the radio choice every six months, from scratch |
| **The mission is the tiebreaker** | Ambiguous calls get made by whoever is most recently annoyed |

The first three are enforced by tooling — the guard hook blocks hand-editing a
tracked generated file, and `fid derive --check` fails CI on staleness. The last
two are enforced by habit, which is why they are written down.

---

## What this is not

**It is not a framework.** There is no runtime you inherit from. Every generated
artifact is human-readable and every high-level declaration has a documented
override. If you need to drop a level, you drop a level without leaving the
system — an abstraction you cannot escape is a trap, and the first genuinely hard
problem will find its edge.

**It does not rebuild what exists.** Not a CAD kernel, not an EDA suite, not a
browser engine, not a cloud. It stands on existing tools and owns the
*integration* between them, because that integration is where the cost is and
where nobody else is working.

**It is not trying to be general.** Nothing is added speculatively. A capability
enters when a real product needs it and is generalized when a *second* one does.
Shipping products is the point; the platform is what is left over from having
shipped them.

---

## Where to go next

| You want to | Read |
|---|---|
| Build something, step by step | [Your first product](./first-product.md) |
| Work on this as an AI agent | [For agents](./for-agents.md) |
| Reuse a codebase you already built | [Harvesting](./harvesting.md) |
| Understand the reasoning in full | [`MISSION.md`](../../MISSION.md) |
| See how the layers fit | [`ARCHITECTURE.md`](../../ARCHITECTURE.md) |
