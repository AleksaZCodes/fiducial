# @fiducial/cli

## 0.3.0

### Minor Changes

- 560b2db: `fid add` edits `fiducial.toml` instead of re-serialising it.
  
  `fiducial.toml` is the one file in a product where a human writes down the
  judgments nothing can derive — which KV namespace id is load-bearing, why a
  Worker name matches an existing deploy by declaration rather than coincidence,
  what an artbox actually measures. Principle 1b says a judgment that cannot be
  derived is still declared, and in practice those declarations are the comments
  around the values.
  
  `patch_config` read the file into the typed `Config`, mutated it, and wrote
  `toml::to_string_pretty` back. That round-trip has nowhere to put a comment, so
  **every `fid add` silently deleted the whole authored commentary** — and
  reflowed the file besides: `vars = { SPOTS_DEFAULT = "14" }` became a
  `[deploy.vars]` table, and every block re-sorted alphabetically out of the order
  someone had put them in. One `fid add seo` destroyed 27 comment lines in a
  product. Every value survived and nothing failed, which is why it went
  unnoticed: the loss is invisible unless you diff a file you did not expect the
  command to rewrite.
  
  The function's own doc comment claimed it "uses line-level editing rather than a
  full TOML rewrite to preserve comments and formatting". It had not done that for
  some time.
  
  It now edits a `toml_edit::DocumentMut`: untouched bytes stay byte-identical,
  and only the arrays and tables being changed are rewritten. `toml_edit` is the
  same parser `toml` 0.8 already uses underneath, so this adds no new transitive
  dependency. The typed `Config` is still loaded first, purely to validate — an
  invalid `fiducial.toml` should fail before it is edited, not after.
  
  Three smaller things fall out of it:
  
  - An array written one-per-line stays one-per-line, and an empty
    `enabled = []` adopts that shape as it fills, which is what the old
    serialiser produced.
  - The `# fiducial.toml — updated by …` header is **replaced**, not prepended.
    It had only ever appeared once because the round-trip deleted the previous
    one along with everything else; preserving comments meant the headers would
    have stacked, each claiming to describe the file.
  - A seeded declaration block gets a blank line before it rather than opening
    flush against the previous block's last value.
  
  Covered by `tests/config_is_authored.rs`. Three of its six cases fail against
  the old implementation.
- 0557d61: Make the `seo` capability work outside the product it was written against.
  
  It shipped with no test, and three defects were sitting in it — each one
  invisible on Next.js and fatal anywhere else.
  
  **Output paths are the pipeline's declaration.** The deriver hard-coded
  `apps/web/public/…` while `pipelines/seo.toml` also declared it. The documented
  one-line change for a SvelteKit product — `public` → `static` — was therefore
  necessary and insufficient: the pipeline guarded a path the script never wrote,
  so `fid derive --check` failed on a *missing* artifact and pointed at neither
  the cause nor the fix. The script now reads its three output paths out of its
  own pipeline, so that edit is enough.
  
  **No undeclared writes.** It imported `parseToml` from `derive-content.mjs`, a
  pipeline executor with no main-module guard, so running the `seo` pipeline
  re-derived the entire content model as an import side effect — writing
  `generated/content.ts`, which `seo` does not declare as an output. Nothing ever
  failed, because content's own pipeline writes the same bytes; the bug was a file
  changing under a pipeline that never claimed it. It now imports the parser from
  `toml-lite.mjs`, which is a parser and nothing else.
  
  **The sitemap-collision guard matches a file name.** It compared against the
  literal `apps/web/public/sitemap.xml`, so it went quiet in exactly the products
  whose paths differ — the ones that had to edit the pipeline, and therefore the
  population most likely to have left both `brand` and `seo` owning the file. Two
  pipelines writing one artifact is a race settled silently by pipeline name
  order.
  
  **`[seo] pages` carries its own message keys.** The script held a table of four
  paths — `/`, `/press`, `/blog`, `/credits` — with one product's key spellings
  baked in, and any other page set hit "no title is declared for it". That named
  the wrong problem: the page had a title, under a key the script had never heard
  of. A page may now declare where its copy lives:
  
  ```toml
  [seo]
  pages = [
      { path = "/", title = "meta.title", description = "meta.description" },
      { path = "/workshop", title = "workshop.heading", description = "workshop.blurb" },
  ]
  ```
  
  A bare path still resolves through the conventional keys, and an unrecognised
  one now prints the declaration to write instead of a misleading diagnosis. This
  also removes the pressure to satisfy a hard-coded key by writing a string a
  second time, which is the duplicate the platform exists to prevent.
  
  `seo` now declares `brand` among its requirements, which it always needed: every
  URL it emits starts with `[brand] domain`.
  
  Covered by `tests/seo_pipeline.rs` — nine end-to-end cases, one per defect.

## 0.2.0

### Minor Changes

- 0894e7c: Add `@fiducial/cli` npm shim and Phase 3b capability mechanism.
  
  **`@fiducial/cli`** — new package. Node shim that resolves `fid` from `FID_BIN` or PATH and delegates to the native binary. Enables `npx @fiducial/cli new my-product` without installing Rust.
  
  **`fid` CLI (fiducial-cli v0.1.0)** — new binary crate.
  
  - `fid new <name>` — scaffold a product repository: `fiducial.toml`, `fiducial.lock`, `MISSION.md`, `AGENTS.md`, `.claude/settings.json` (guard hook), `.gitignore`, `README.md`, `git init`.
  - `fid add app next|tauri|worker` / `fid add firmware rp2040` — install built-in capabilities: write template files, activate guard rules in `fiducial.toml`, install `SKILL.md` into `.claude/skills/<id>.md`.
  - `fid capability list [--all]` / `fid capability check` / `fid capability new <name>` — manage capabilities.
  - `fid doctor` — verify `fiducial.toml`, `fiducial.lock`, and SHA-256 template integrity.
  - `fid guard-check` — shell-aware `PreToolUse` hook; tokenizes Bash commands before matching guard rules so `echo "npm install"` never triggers the `npm` rule.
  - Stub commands with full `--help`: `fid derive`, `fid upgrade`, `fid graph` (Phase 4).
  - Built-in capabilities: `web-next` (Next.js 15), `firmware-rp2040` (Embassy), `tauri`, `worker-cloudflare`.
  - 11 guard unit tests; `fiducial.lock` re-baselined after every capability install so `fid doctor` stays clean.
