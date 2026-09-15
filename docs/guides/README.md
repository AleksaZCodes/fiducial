# Guides

| Guide | For |
|---|---|
| [**Start here**](./start-here.md) | What this system is and why it is shaped that way. Read first. |
| [**Your first product**](./first-product.md) | Nothing → a board, a generated enclosure, a CI gate. ~20 minutes. |
| [**For agents**](./for-agents.md) | Working in a Fiducial product as an AI agent. |
| [**Harvesting**](./harvesting.md) | Getting the good parts out of a codebase you already built. |

## Reference

| Document | Holds |
|---|---|
| [`MISSION.md`](../../MISSION.md) | The reasoning. The tiebreaker for ambiguous decisions. |
| [`ARCHITECTURE.md`](../../ARCHITECTURE.md) | How the layers fit together. |
| [`STACK.md`](../../STACK.md) | Every technology choice, enumerated. |
| [`ROADMAP.md`](../../ROADMAP.md) | What is intended, in order — and so what is next. |
| [`SHIPPED.md`](../../SHIPPED.md) | What was built, phase by phase. |
| [`docs/protocol/`](../protocol/) | The wire specification and conformance vectors. |
| [`docs/specs/`](../specs/) | Design decisions, append-only, date-stamped. |
| [`docs/harvest/`](../harvest/) | Catalogues of donor codebases. |

## About the terminal output in these guides

Every block showing `fid` output is **generated from the real binary** and
verified by CI. They are not pasted.

A hand-pasted terminal block is a derived artifact maintained by memory: correct
the day it is written, silently wrong afterwards, with no way for a reader to
tell. Writing one into the documentation of a platform built to delete exactly
that failure would be indefensible.

So `docs/captures/` holds the real output, regenerated with:

```sh
FIDUCIAL_WRITE_CAPTURES=1 cargo test -p fiducial-cli --test captures
```

CI runs the same test **without** that variable. A change to `fid`'s output that
skipped regeneration fails the build rather than leaving a guide that lies.
