# Phase 22 — i18n: what is done, what is left

> **To finish this phase:** read this file, then work the checklist. Everything
> needed is here; no conversation context is required.
>
> Delete this file when the phase closes. It is a handoff, not documentation.

---

## The goal

`MISSION.md` principle **1c** — *a user-visible string is a fact, declared once,
with every locale a derivation that must exist.* A missing translation is a
missing artifact, not a fallback, and it fails the build.

Full design and reasoning: **[`ROADMAP.md`](../ROADMAP.md)**, section
*"1 · i18n — localized by construction"*.

## Decisions already made — do not relitigate

| Decision | Value |
|---|---|
| Catalog format | **JSON**, `messages/<locale>.json`, one file per locale. Nested or flat |
| Why JSON | A translator must be able to edit it without a build system |
| Default locales | **`sr` and `en`** |
| Hardcoded-string detection | **Warns, does not fail.** But *reported*, not printed — see below |
| Rust key catalog | **Not now.** TypeScript is the first derivation; JSON stays the declaration so Rust can be added later as a *new derivation* |

## Done

**`packages/i18n`** — the runtime. 43 tests, all green.

- `locale.ts` — `Accept-Language` parsing with q-values, two-pass negotiation
  (exact tag, then primary subtag), cookie precedence
- `translate.ts` — strict lookup. Default `onMissing` **throws**; production
  passes a reporter. Falls back to the default locale *and reports*, because the
  defect in Ring of Pursuit was the silence, not the fallback
- `money.ts` — integer minor units, currency independent of locale,
  cross-currency arithmetic refuses, `allocate()` loses nothing
- `format.ts` — dates, timezones (offset-probing inverse, DST-verified), numbers,
  plurals (Serbian has three categories), relative time, lists

**`crates/fiducial-cli/src/i18n.rs`** — the pipeline logic. 12 unit tests, green.

- `load_catalog` — flattens nested JSON to dotted keys
- `compare` — **fatal:** missing key, placeholder mismatch. **Reported:** extra
  key, value identical to default and longer than 12 characters
- `render_typescript` — the `MessageKey` union, `Locale` type, per-locale records

⚠️ **This module carries `#![allow(dead_code)]` because nothing calls it yet.**
Remove that line as part of the wiring — it is the marker for this handoff.

## Left to do

### 1 · Wire the `fid-i18n` executor

`crates/fiducial-cli/src/commands/derive.rs`. Follow `run_fid_validate` and
`run_fid_mesh` exactly — same shape, same dispatch site (~line 355, the
`match pipeline.executor.as_str()` arm).

```
pipeline:  name = "i18n", executor = "fid-i18n"
           args = ["messages"]                      # catalog directory
           outputs = ["packages/…/generated/messages.ts"]
```

Behaviour: load every `messages/*.json`, `compare` against the default locale,
**bail on `findings.is_fatal()`**, otherwise write `render_typescript` output to
the declared output path. Remove `#![allow(dead_code)]`.

The error must name the locale, the key, and the file to edit — every other
error in this codebase does.

### 2 · The `[i18n]` declaration

`fiducial.toml`:

```toml
[i18n]
locales = ["sr", "en"]
default = "sr"
```

Read it in `config.rs` (see `Spine`/`Capabilities` for the pattern). The default
locale is **not** `locales[0]` — state it, because inferring it is the kind of
implicit fact this platform exists to delete.

### 3 · `fid add i18n` capability

`crates/fiducial-cli/src/capability.rs` — copy the `eda` entry. Templates:

- `messages/sr.json`, `messages/en.json` — a couple of seed keys
- `pipelines/i18n.toml`
- `SKILL.md` — **mandatory.** Cover: keys are named, never English text; a
  missing translation fails the build; plurals belong in the catalog because
  Serbian has three categories; how to clear detector findings

Installing must add `[i18n]` to `fiducial.toml`, like `eda` adds its guard rule.

### 4 · The hardcoded-string detector

**Warns, never fails.** But a warning is a log line and log lines are ignored by
humans and agents alike — so it is *reported*:

- a count and `file:line` list in **`fid doctor`**
- a section in **`fid dash`**, and in **`fid dash --json`** so an agent consumes
  it as data
- the `SKILL.md` tells agents to clear them opportunistically

Scan `.tsx` / `.jsx` / `.svelte` / `.html` for user-visible literals not passing
through `t()`. **Expect false positives** — CSS classes, `data-testid`, aria
roles, URLs, `className`. That imprecision is exactly why it warns rather than
fails. Provide an opt-out marker (`// i18n-ignore`) for genuine exceptions.

### 5 · `fid new` takes locales

Default `sr,en`. Scaffold `messages/*.json` and the `[i18n]` block. There must be
no moment where a product is monolingual — that is the whole mechanism behind
"second nature" rather than "a refactoring pass".

### 6 · Close out

- `FIDUCIAL_WRITE_CAPTURES=1 cargo test -p fiducial-cli --test captures` — the
  `fid new` capture will change
- Add crate/package to the layout trees in `CLAUDE.md` **and** `AGENTS.md` — a
  hygiene test enforces this
- A changeset for `@fiducial/i18n` if its API moved
- Mark i18n ✅ in `ROADMAP.md`, add the row to `SHIPPED.md`
- **Delete this file**

## House rules that will trip you up

- **Commit subjects ≤ 72 chars, Conventional Commits.** Gated by
  `tests/commit_hygiene.rs`
- **Never edit a file in `docs/specs/`.** Append-only. Write a new dated one
- **Principles are authored only in `MISSION.md`** — `build.rs` generates them
  into the scaffolded `AGENTS.md`
- `ROADMAP.md` = intended · `SHIPPED.md` = built. Do not name next steps in
  `SHIPPED.md`
- Run `cargo test --workspace --all-features` — `--all-features` matters, the
  README doctests need it

## Verify before opening a PR

```sh
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets   # CI treats warnings as errors
cargo test --workspace --all-features                   # 385 passing at handoff
pnpm -s turbo build typecheck test                      # 29 tasks
```
