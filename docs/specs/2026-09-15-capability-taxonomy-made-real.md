# Capability taxonomy, made real

**Date:** 2026-09-15
**Status:** implemented
**Extends:** [`2026-09-14-capability-taxonomy.md`](2026-09-14-capability-taxonomy.md),
which is unchanged. That spec decided the three concepts; this one records what
implementing them found, and one decision it did not cover.

---

## What the split actually removed

Before this, a capability was a flat list of files:

```rust
templates: &[
    ("messages/en.json",     …),   // a declaration
    ("pipelines/i18n.toml",  …),   // a pipeline
    ("apps/worker/wrangler.toml", …), // a tool's config file
]
```

Three different kinds of thing, indistinguishable to every consumer. The cost
was not theoretical — it was one `if`:

```rust
if cap.id == "i18n" && cfg.i18n.locales.is_empty() {
    cfg.i18n.locales = …;
}
```

`patch_config` had to know that the `i18n` capability introduces an `[i18n]`
block, because the capability had no way to say so. The next capability with a
config block would have added a second branch to the same function, and the one
after that a third.

`Declaration::ConfigBlock { name, seed }` carries a `fn(&mut Config)`, so the
capability that owns the fact owns filling it in. The branch is gone.

## Conformance is checked, not assumed

A split nothing enforces collapses back. `check_capability` now rejects:

- a pipeline outside `pipelines/` — `pipeline::discover` looks there, so one
  filed elsewhere installs cleanly and never runs;
- a file under `pipelines/` declared as a template — installed, but not gated by
  `fid derive --check`, which is the only property that makes a pipeline one;
- **a capability with pipelines and no declarations** — a pipeline with no
  declared input reads a fact nobody is responsible for putting there;
- a required adapter contract that does not exist.

`every_builtin_capability_conforms` runs those against the registry itself, so a
first-party capability cannot ship violating a rule the CLI enforces on
everyone else's. Both checks were verified by breaking a real capability and
watching them fail.

## The decision the first spec did not make: what ships in `[adapters]`

The accepted spec lists example vendors — `d1`, `supabase`, `neon`, `r2`, `s3`,
`resend`. It also warns, in the same section, that *"a badly drawn contract is
worse than no abstraction — it is lock-in wearing a portability costume."*

Nothing implements any of those vendors yet. Roadmap item 6 builds the first
set. So shipping a registry that lets a product write `database = "supabase"`
would mean a selectable name with nothing behind it — **the identical bug this
repository removed from its guard rules the day before**, where
`no-direct-main-push`, `no-hand-edit-generated` and three other rule names were
declared, counted by `fid dash`, and enforced by nothing.

**A selectable vendor is a promise.** So a contract carries two lists:

| List | Means | Selecting one |
|---|---|---|
| `implementations` | works today | fine |
| `candidates` | intended, unbuilt | fails, and says it is a *not yet* |

Today every contract implements exactly `none` — a real, working no-op, not a
placeholder. `errors = "none"` is diagnostics wired in from the first commit,
costing nothing until pointed somewhere, because a capability you have to
retrofit is one you will not retrofit.

The two failure messages are deliberately different. `supabase` reads as *"names
an intended vendor that nothing implements yet"*; `supabse` reads as *"not a
known implementation"*. A roadmap item and a typo need different answers, and a
single "unknown value" message gives the same answer to both.

## What is reported

`fid dash --section taxonomy` lists every declaration with whether it is
**present**, and every contract with what satisfies it — including contracts
nothing selected, because an unfilled contract is a fact about the product and
listing only what was chosen hides it. An `[adapters]` key that is not a
contract at all is appended to the listing rather than silently dropped, since
the loop walks known contracts and would otherwise show it nowhere.

`fid doctor` treats an unresolvable selection as an **issue**, not a warning:
unlike a hardcoded string, there is no judgment involved. `fid capability list
--all` prints the contracts, because the set lives in the binary and a product
cannot select a vendor it has never been told exists.

## Deliberately not done

- **No real adapter implementations.** That is roadmap item 6, and doing it here
  would design contracts against no consumer — the failure mode the first spec
  names.
- **No adapter *capability*.** A contract is not something you `fid add`; it is
  a slot a product fills. If a vendor implementation later needs files, that
  implementation becomes a capability and the contract stays a contract.
- **`Config::adapters` is a map, not a struct with a field per contract.** A
  struct would be a second copy of `adapter::CONTRACTS` and would drift the
  first time one was added.
