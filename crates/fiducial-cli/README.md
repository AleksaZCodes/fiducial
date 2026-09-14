# fiducial-cli

**`fid` — the command line for the Fiducial platform.**

[![crates.io](https://img.shields.io/crates/v/fiducial-cli.svg)](https://crates.io/crates/fiducial-cli)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## Install

```sh
cargo install fiducial-cli
```

## Start a product

```sh
fid new my-product
cd my-product
fid dash          # roadmap, decisions, CI, pipelines, freshness — one view
```

## Commands

| Command | Does |
|---|---|
| `fid new <name>` | Scaffold a product: config, lock, mission, agent context, CI, guard |
| `fid add app next\|svelte\|tauri\|worker` | Install a web/desktop/edge capability |
| `fid add firmware rp2040\|stm32` | Install a flashable firmware scaffold |
| `fid add eda` | Install the board pipeline and enclosure generation |
| `fid add component <name>` | Copy a UI component into the product, source and all |
| `fid harvest <path>` | Survey an existing codebase for reusable logic, art, UI and principles |
| `fid derive` | Run the pipelines; record every artifact hash in the lock |
| `fid derive --check` | Fail when an artifact is stale. **This is the CI gate.** |
| `fid graph` | Print the pipeline DAG as text, dot or json |
| `fid doctor` | Verify config, lock, template integrity, pending migrations |
| `fid upgrade` | 3-way merge upstream template changes; apply codemods |
| `fid dash [--json] [--portfolio]` | The workbench view |
| `fid release check\|status\|protocol` | Wire-version policy and skew enforcement |
| `fid capability list\|check\|new` | Inspect and author capabilities |
| `fid guard-check` | PreToolUse hook — shell-aware guard rule enforcement |

Every command has full `--help`. **The help text names no phase**, deliberately:
a schedule is a fact `PHASES.md` owns, and a second copy of it drifts. A test
enforces that, and CI runs it.

## Reusing what you already built

```sh
fid harvest ~/dev/old-web-app --name oldapp   # survey + stage
/fiducial:harvest oldapp                      # extract, with judgment
```

The command does the mechanical half — walk, classify, measure, detect the
donor's stack, stage readable copies. The skill does the half that is judgment:
whether a function is a business rule or incidental framing is not a heuristic.

Everything lands in `harvest/<name>/`, which is **not** your source tree. Nothing
is wired in. See [the guide](../../docs/guides/harvesting.md).

## Three things worth knowing

**`fid dash` owns no store.** Every number is recomputed per run from files
already in the repository, and a test asserts the command writes nothing at all.
It makes no network calls — CI status is read from declared workflow files, and a
test renders the whole dashboard with every proxy pointed at a closed port. A
view that keeps its own copy has to be kept in sync, and keeping things in sync is
the work this system exists to delete.

**The guard is shell-aware.** `fid guard-check` tokenizes a command before
matching, so a rule fires in command position and never inside a quoted string or
an argument. This matters: the naive version flagged its own documentation.

**`fid upgrade` is a real 3-way merge.** The base comes from `fiducial.lock`,
"ours" from disk and "theirs" from the binary, so an upstream template change
lands in a product you have already edited, and a genuine conflict is surfaced
rather than overwritten.

## License

MIT
