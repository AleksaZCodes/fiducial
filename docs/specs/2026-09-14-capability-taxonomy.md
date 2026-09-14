# Capability taxonomy — declaration, pipeline, adapter

**Date:** 2026-09-14
**Status:** accepted
**Supersedes:** the single `CapabilityDef` concept from Phase 3b (which remains
the shipping unit, but stops being the only concept).

---

## Context

Phase 3b introduced a capability:

```rust
struct CapabilityDef {
    id, description,
    guard_rules,   // rules that activate
    templates,     // (path, content) files copied into the product
    skill_md,      // instructions for agents
}
```

**A capability is a bundle of files that get copied in, plus a skill file.** That
served `web-next`, `firmware-rp2040`, `tauri` and `eda` well, because each of
those genuinely is "copy these files in and turn some rules on."

It stops serving the moment the question is asked directly: *is brand a
capability?*

Brand is not files to copy. Brand is one set of facts — legal name, contact
email, domain, palette, logo — that **many unrelated things read** in order to
produce favicons, OG images, a press kit, email themes and legal pages. No
arrangement of `templates: &[(&str, &str)]` expresses that.

The same is true of locales, and of the choice between Supabase and D1. The word
"capability" had been carrying three jobs.

## Decision

Split it into three concepts, and keep "capability" as the shipping unit that
bundles them.

### 1 · Declaration

Typed facts, written once, **inert**. A declaration does nothing by itself.

`[brand]`, `[locales]`, `[adapters]` in `fiducial.toml`;
`board/board.interface.json`.

The test for a declaration: *could two different pipelines read this and both be
correct?* If yes, it is a declaration rather than one pipeline's config.

### 2 · Pipeline

Reads declarations, produces artifacts, records their hashes in `fiducial.lock`,
and is gated by `fid derive --check`.

This already exists (`pipelines/*.toml`, the `fid-validate` and `fid-mesh`
executors). What changes is that pipelines become something a **capability can
install**, rather than something only the EDA capability happens to have.

The property that matters is the gate: a pipeline's output is *derived*, so it
going stale is a build failure rather than a surprise.

### 3 · Adapter

A swappable implementation behind a fixed contract. This is `MISSION.md`
principle 6 — *commit to contracts, not to tools* — which until now existed only
as prose.

```toml
[adapters]
database = "d1"          # or "supabase", "neon", "mongo"
storage  = "r2"          # or "s3", "supabase-storage"
deploy   = "cloudflare"  # or "vercel"
email    = "resend"      # or "ses", "cloudflare-email"
errors   = "none"        # a no-op default that is still wired in
```

The contract is fixed and vendor-neutral. Swapping vendors is editing one line.

**The honest cost, recorded so it is not rediscovered:** a contract must be
designed against the *narrowest* plausible vendor or it leaks that vendor's
model into every consumer. Adapters are the highest-value and highest-effort
item in the roadmap, and a badly drawn contract is worse than no abstraction —
it is lock-in wearing a portability costume.

**The no-op default matters.** `errors = "none"` means diagnostics are wired in
from the first commit and cost nothing until pointed somewhere. A capability you
have to retrofit is one you will not retrofit.

### 4 · Capability — the shipping unit

A capability bundles any of the above, plus guard rules and a skill:

```
capability
  ├── declarations   the typed facts it introduces
  ├── pipelines      what it derives from them
  ├── adapters       the contracts it satisfies or requires
  ├── templates      files copied in (as today)
  ├── guard rules    what becomes forbidden
  └── SKILL.md       how an agent should use it
```

**Granularity:** a capability is as big as one decision someone would actually
make. "I want a web app", "I want legal pages", "I want the product localized"
are decisions. "I want a button" is a component, not a capability. The practical
test — *would anyone install this on its own?*

## Consequences

- `fid add <capability>` may now introduce a declaration and a pipeline, not just
  files. `fid derive` picks up the pipeline automatically.
- `fid dash` can report declarations present, pipelines wired, adapters selected.
- A capability can *require* an adapter contract without choosing the vendor.
- **Capabilities must become resolvable from outside the binary.** Today every
  one is `include_str!`'d into `fid`, so publishing a capability requires
  releasing the CLI. That blocks third-party capabilities entirely — including
  first-party ones we would rather ship on their own cadence. Tracked as
  roadmap item 24.

## What this does not change

The existing four capabilities keep working unchanged. This is additive: a
capability with only `templates` and a `SKILL.md` remains valid, because that is
genuinely all some of them need.
