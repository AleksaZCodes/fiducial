# Context sync — derive the agent context that is derivable

**Date:** 2026-09-15
**Status:** implemented
**Implements:** the `ROADMAP.md` item **context sync**.

---

## The objection this is built on

The roadmap states the problem and rejects the obvious fix in the same breath:

> `CLAUDE.md` and `AGENTS.md` both carried repository trees that had gone stale
> past three crates, *and both carried a disclaimer telling the reader to run
> `ls` instead of trusting them.* Phase 18 fixed them by hand and added a test.
> **A test that fails after the fact is detection, not sync.**

That test — `agent_context_layout_tree_names_every_crate_and_package` — worked.
It also meant that adding a crate turned the build red until someone edited a
tree by hand, which is a tax on every later phase rather than a fix.

## The line drawn

| Derivable — generated | Judgment — hand-written |
|---|---|
| the `crates/` and `packages/` tree, with each member's own `description` | what not to do, and why |
| every `fid` command, from the binary's own `clap` definition | how to think about a capability |
| every built-in capability and what it contributes | which rules exist and what they defend |
| where the instructions an agent can load live | everything in `MISSION.md` |

The test is whether a machine can answer it from a declaration that already
exists. A crate's one-line summary is its `description` — the field crates.io
and npm already show. Writing a second summary into `AGENTS.md` is exactly how
the tree went stale.

## Markers, and why HTML comments

```markdown
<!-- fid:begin layout -->
…generated…
<!-- fid:end layout -->
```

They render as nothing on GitHub and in every Markdown viewer, so a generated
block does not announce itself to a human reader who only wants the content.

**A file with no markers is left completely alone.** This is opt-in per file
*and* per block, which is what lets a product adopt the layout block without
surrendering its whole `AGENTS.md` to a generator.

**A marker naming a block the generator does not know is an error**, not a
no-op. Silently skipping it leaves a block that looks generated, is actually
frozen, and drifts — the worst of both.

## Why a command rather than a pipeline

`fid derive` runs a *product's* declared pipelines and records their outputs in
`fiducial.lock`. Agent context is not a product artifact: it exists in the
platform repository too, which deliberately declares no pipelines — its
`fiducial.toml` says why, and the reason is that a gate running through the tool
it gates is blind exactly where it matters.

So `fid context` and `fid context --check` are the same freshness contract in
their own command, gated in CI alongside the captures.

## What this replaced

`agent_context_layout_tree_names_every_crate_and_package` is gone. In its place,
`agent_context_is_generated_rather_than_asserted` checks that the four blocks
are still *marked* — because a file that quietly lost its markers would go back
to being hand-written and stale with nothing saying so.

Detection became prevention, and the remaining test guards the mechanism rather
than the content.

## Deliberately not done

- **`CLAUDE.md` is left hand-written.** It is 58 lines and every one of them is
  Claude-specific judgment — which skills exist, why the subagents are
  namespaced. There is nothing derivable in it to generate.
- **No generated prose.** A block is a tree or a table. Generating sentences
  produces text that reads like it was written by nobody, which is worse than a
  slightly stale sentence written by someone.
- **The product `AGENTS.md` template is not marked yet.** Scaffolded products
  would need the blocks to describe *their* layout, which works, but no product
  has asked and marking it commits every future product to the generator. The
  rule of two applies to the platform's own features too.
