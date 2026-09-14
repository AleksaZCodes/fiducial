# Your first product

> From nothing to a board, a generated enclosure, and a CI gate that catches
> drift. About twenty minutes.

Assumes you have read [Start here](./start-here.md). No hardware required —
every pipeline in this guide runs in software.

---

## 0 · Install

```sh
cargo install fiducial-cli
fid --version
```

## 1 · Scaffold

```sh
fid new demo-product
cd demo-product
```

<!-- capture: fid-new.txt -->

```text
$ fid new demo-product
✦ fid new demo-product
  wrote  fiducial.toml
  wrote  MISSION.md
  wrote  AGENTS.md
  wrote  README.md
  wrote  .gitignore
  wrote  .claude/settings.json
  wrote  .claude/agents/fiducial-review.md
  wrote  .claude/agents/fiducial-design.md
  wrote  .github/workflows/ci.yml
  wrote  .github/workflows/claude-review.yml
  wrote  fiducial.lock
Initialized empty Git repository in /home/you/dev/demo-product/.git/
  git    init (branch: main)

✦ demo-product is ready. Next steps:

  cd demo-product
  # Edit fiducial.toml — set spine.enabled = true if you want the L0 Rust core.
  # Edit MISSION.md   — one paragraph: what is this product for?

  fid add app next      # add a Next.js web app
  fid add app svelte    # add a SvelteKit app
  fid add app tauri     # add a Tauri desktop/mobile app
  fid add firmware rp2040  # add RP2040 firmware

  fid doctor            # verify everything is in order

  # Agents — Sonnet for implementation, Opus for design/review:
  fiducial-design       # architecture brainstorming
  fiducial-review       # review diff before committing
  # CI auto-reviews every PR (needs ANTHROPIC_API_KEY in repo secrets).

  # When you're ready to commit:
  git add -A && git commit -m 'feat: initial scaffold'
```

<!-- /capture -->

Nine files. The ones that matter:

| File | Is |
|---|---|
| `fiducial.toml` | Your product's declaration: name, capabilities, guard rules |
| `fiducial.lock` | Hashes of every template and every generated artifact |
| `MISSION.md` | Copied verbatim, not per-product. The tiebreaker. |
| `AGENTS.md` | Context for AI agents working in this repo |
| `.github/workflows/ci.yml` | Runs `fid doctor` and `fid derive --check` |

That last one is the point of the whole system, and it is scaffolded rather than
left as something to remember. Note the branch is `main` — the guard rule, the
review agent and the workflow all reference it, so it would be incoherent to
start on `master`.

## 2 · Check it

```sh
fid doctor
```

<!-- capture: fid-doctor.txt -->

```text
$ fid doctor
✦ fid doctor — /home/you/dev/demo-product

  ✓ fiducial.toml valid  (product: demo-product, vX.Y.Z)
  ✓ fiducial.lock valid  (10 template(s) tracked, 0 migration(s) applied)
  ✓ all template files unmodified
  ✓ templates up to date with platform vX.Y.Z
  ✓ no pending codemod migrations

✦ fiducial doctor: clean
```

<!-- /capture -->

`doctor` answers a narrow question: *is this product internally consistent?* It
verifies the config parses, the lock matches what is on disk, no template has
been hand-edited, and no codemod is pending.

## 3 · See the whole thing

```sh
fid dash
```

The one command worth learning. Seven sections, every number recomputed from
files already in the repo. Run it casually — it exits 0 even when it reports
findings, because gating is `fid derive --check`'s job and a dashboard you are
afraid to run is a dashboard you do not run.

Look at the CI section specifically:

```sh
fid dash --section ci
```

<!-- capture: fid-dash-ci.txt -->

```text
$ fid dash --section ci

CI
  CI                           on push, pull_request  [checks artifact freshness]
  Claude Review                on pull_request
  (declared workflows, not live run status — dash makes no network calls)
```

<!-- /capture -->

`[checks artifact freshness]` is dash confirming something real: a workflow in
this repo runs `fid derive --check`. If you deleted that job, this line would
change to a warning — the dashboard reports the absence of the guard, not just
its presence.

## 4 · Declare a board

```sh
fid add eda
```

That installs two pipelines and a seed declaration at
`board/board.interface.json`. Open it. This is the file the rest of the guide
derives from:

```json
{
  "schema_version": "1.0",
  "board": { "name": "demo", "revision": "A" },
  "outline": {
    "width_mm": 100.0,
    "height_mm": 60.0,
    "thickness_mm": 1.6,
    "tolerance": "fdm",
    "enclosure": { "headroom_mm": 10.0, "standoff_height_mm": 3.0 }
  },
  "connectors": [
    {
      "id": "J1", "name": "USB-C", "type": "usb-c",
      "mount": { "side": "south", "offset_mm": 20.0 },
      "pins": [{ "number": 1, "name": "VBUS", "net": "PWR_5V", "direction": "power_in" }]
    }
  ]
}
```

Three things here are doing more work than they look like they are:

**`"tolerance": "fdm"`** declares the manufacturing process. Clearances and wall
thicknesses are derived from it, so the same board yields a *tighter* case on
resin than on FDM without you editing a dimension.

**`"type": "usb-c"`** implies the size of the hole. The connector family maps to
a body envelope, so the opening is never typed in a second time. Change the
connector type and the hole changes.

**`"offset_mm": 20.0`** is in board coordinates, so it means the same thing on
every edge. Move the connector to `"side": "west"` and it stays 20 mm from the
same origin.

## 5 · Derive

```sh
fid derive
```

This runs both pipelines and writes:

```
enclosure/case-base.stl   the base, with a gasket groove and the USB-C opening
enclosure/case-lid.stl    the lid, with a compression tongue
enclosure/gasket.stl      the seal — print this in TPU
enclosure/case.glb        the exploded assembly, for a web viewer
```

**Nobody modelled any of that.** It came from the declaration in step 4.

See how the pieces connect:

```sh
fid graph
```

<!-- capture: fid-graph.txt -->

```text
$ fid graph
pipeline: eda (fid-validate)
  → artifact: board/board.interface.json
pipeline: enclosure (fid-mesh)
  → artifact: enclosure/case-base.stl
  → artifact: enclosure/case-lid.stl
  → artifact: enclosure/gasket.stl
  → artifact: enclosure/case.glb
```

<!-- /capture -->

## 6 · The part that matters

Check freshness:

```sh
fid derive --check
```

<!-- capture: fid-derive-check.txt -->

```text
$ fid derive --check
✦ fid derive --check — all artifacts fresh
```

<!-- /capture -->

Now break it deliberately. Open `board/board.interface.json`, change
`width_mm` to `120.0`, and run `--check` again:

```
✗ enclosure/case-base.stl — STALE
  the declaration changed but the artifact was not regenerated
```

Non-zero exit. **That is the guarantee.** Not "we generate things" — an artifact
that has drifted from its declaration cannot reach `main`, because the workflow
scaffolded in step 1 runs exactly this.

Run `fid derive` and it is green again — and the case is now 10 mm wider, with
the USB-C opening still 20 mm from the same edge.

## 7 · Version the wire

If your product talks to a device, the protocol version is a fact too:

```sh
fid release status
```

<!-- capture: fid-release-status.txt -->

```text
$ fid release status
✦ fid release status

  platform cli    X.Y.Z
  wire version    2  (WIRE_VERSION in fiducial-protocol)

  ⚠ could not read docs/compat/matrix.toml: reading /home/you/dev/demo-product/docs/compat/matrix.toml
    Create it with `fid release protocol --bump breaking`.
```

<!-- /capture -->

`fid release check` fails when the committed compatibility matrix and the
compiled `WIRE_VERSION` disagree — so a protocol bump that skipped the matrix
fails CI rather than shipping a firmware that cannot talk to its app.

---

## What you just built

```
board/board.interface.json     ← you wrote this
         │
         │  fid derive
         ▼
enclosure/*.stl   enclosure/*.glb   packages/*/types.ts
         │
         │  fid derive --check   (in CI, on every push)
         ▼
   a build that fails if any of them stopped matching
```

One declaration. Four derivations. A gate that catches the fifth time you forget.

## Where to go next

| To | Do |
|---|---|
| Add a web app | `fid add app next` or `fid add app svelte` |
| Add firmware | `fid add firmware rp2040` or `stm32` |
| Add a desktop app | `fid add app tauri` |
| Reuse an old codebase | [Harvesting](./harvesting.md) |
| Work with agents | [For agents](./for-agents.md) |

## When something goes wrong

| Symptom | Cause | Fix |
|---|---|---|
| `fid doctor` reports a modified template | A scaffolded file was hand-edited | Keep it, or `fid upgrade` to merge upstream changes |
| `fid derive --check` fails | An artifact drifted from its declaration | `fid derive` — never edit the artifact |
| The guard blocks your edit | You are editing a tracked generated file | Edit its declaration instead |
| `fid dash` says no freshness guard | No workflow runs `fid derive --check` | `fid upgrade` installs the CI template |
