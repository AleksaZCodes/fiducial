---
"@fiducial/cli": minor
---

`fid add` edits `fiducial.toml` instead of re-serialising it.

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
