# External capabilities — a capability is a directory, not a literal

**Date:** 2026-09-15
**Status:** implemented
**Implements:** `ROADMAP.md` item 3, under the source model design spec §3.3
already decided.

---

## The claim being made true

> *"We can add that later, easily" is only true once shipping a capability does
> not require releasing the CLI.* — `ROADMAP.md` item 3

Every capability was a `CapabilityDef` literal in `capability.rs` listing its
files by hand: 56 `include_str!` calls, next to a directory that already
contained exactly those files. Two consequences, one obvious and one not.

The obvious one: a third party could not ship a capability at all.

The other: **adding a file to a first-party capability meant editing two
places.** The directory and the literal were both declarations of the same
thing, and nothing checked that they agreed. A file added to the directory and
not to the literal is a capability quietly missing a file.

## The directory is the declaration

`build.rs` walks `capabilities/<id>/` and emits the embedded bytes;
`manifest::derive` turns bytes into a `Capability` at runtime. A capability
resolved from a git repository goes through **the same derivation** — which is
what makes "not compiled into the CLI" true rather than aspirational, and is
asserted by `a_builtin_derives_the_same_from_its_directory`.

Deliberately, the build script emits **bytes and not a parsed manifest**. A
build script that produced the parsed form would be a second implementation of
the derivation, in a place no test can reach.

## What the layout means

| Path | Becomes | Why the layout can say it |
|---|---|---|
| `SKILL.md` | the skill | **required, and the only required file** |
| `capability.toml` | description, guard rules, required adapters, config-block declarations | the facts a layout cannot carry |
| `declarations/…` | a declaration, installed at the path *below* `declarations/` | a declaration and a template are both copied files; what separates them is not visible in a path, so the path says it |
| `pipelines/*.toml` | a pipeline | already where `pipeline::discover` looks inside a product |
| anything else | a template | |

§3.3 is emphatic that **one file is a complete capability**: *"the failure mode
for an extension system is ceremony."* So `capability.toml` is optional, and a
capability without one takes its description from the first line of prose in its
own skill.

## A config-block declaration had to stop being a function

`Declaration::ConfigBlock` carried `fn(&mut Config)`. No capability outside this
binary can supply a Rust function — the signature quietly made "resolvable from
outside" false for every capability that declares a block, which is most of the
interesting ones.

It is now TOML data merged at the `toml::Value` level, not through the typed
`Config`. Typing it would mean only blocks compiled into `fid` could be
declared, which is the same limitation one layer down.

That change removed a duplicate nobody had noticed: `fid new --locales`
defaulted to a constant in `config.rs` while the `i18n` capability seeded its
own. Two declarations of *the locales a product is born with*. `fid new` now
asks the capability.

## The source model, and what is deferred

`--from <path>` and `--from git:<url>[#<rev>][::<subdir>]`.

§3.3 names the model exactly: capabilities *"resolve from the platform monorepo
(first-party), or any git repo (third-party or private) — the same source model
the Claude Code marketplace already supports."* Git covers public and private,
pins to a commit, and is installed wherever `fid` is.

**npm and crates are deferred**, and the reason is recorded so it is not
re-litigated: a registry adds a packaging format — what a published capability
contains, how a version range resolves, which registry is authoritative — and
nothing needs that yet. The roadmap named them; §3.3 named git; git is the one
with a consumer.

`git:` is a prefix rather than a sniff. A URL and a path are not reliably
distinguishable, and a wrong guess reaches the network.

## A git source pins the commit, never the reference

`--from git:…#main` installs what `main` was at that moment, and the lock
records the **commit**. A lock that recorded `main` would pin a question whose
answer changes.

## Trust, because this is now other people's content

`fid add` writes files into your repository. Everything arrives through
`manifest::derive`, so one set of checks covers built-in, path and git:

- no absolute path, no `..` segment, nothing under `.git`
- the directory's name is the id, and it must be the one asked for — otherwise
  `fid add stripe --from ./billing` records a capability that is not what it says
- **the same conformance checks a built-in passes**, before anything is written.
  A capability arriving from outside is a reason for more checking, not less.
- `deny_unknown_fields` on the manifest: a misspelled key that is silently
  ignored is a capability that does not do what its author wrote down

## The lock had to learn about capabilities

Every consumer looked a capability up in the built-in registry — which silently
stopped being the whole truth the moment a product could install one from a
repository. An externally resolved capability was invisible to `fid dash` and
its adapter requirements went unenforced by `fid doctor`.

`fiducial.lock` now records each installed capability: source, platform version,
a content hash over everything it installs, description, declarations, pipelines
and required adapters. Enough to report it without re-resolving, because
`fid dash` must not reach the network and `fid doctor` must work offline.

## A bug this refactor surfaced

`templates::raw` mapped an installed path to its template through a hand-written
match. `firmware/Cargo.toml`, `firmware/rust-toolchain.toml` and
`firmware/shared/src/lib.rs` are installed by **both** firmware capabilities
with different content, and the match had one arm each, always resolving to the
RP2040 version.

So a product with `firmware-stm32` had `fid doctor` and `fid upgrade` comparing
its files against the wrong board's template — reporting drift that is not
drift, and on upgrade offering to overwrite a correct file with another board's.

`templates::raw_for` resolves against the capabilities the product actually
installed. Where a path is still ambiguous it returns `None` and the caller
skips the file: a comparison against the wrong template is worse than no
comparison.

The match also carried four `firmware-stm32/…` keys that nothing ever looked
up — the callers pass real installed paths. Declared, unreachable, and removed.

## Deliberately not done

- **No capability marketplace or index.** Resolution is by explicit source. A
  discovery layer is a product decision with no consumer yet.
- **No transitive capability dependencies.** No capability needs one, and a
  resolver is the kind of thing that is much easier to add than to remove.
- **Binary files in a capability are refused**, with the path named. Every
  capability today is text; supporting binaries means deciding how they are
  hashed, diffed and merged by `fid upgrade`, which no capability has asked for.
