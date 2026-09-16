# The AI contract targets a gateway, and the model is a declaration

**Date:** 2026-09-16
**Status:** accepted

---

## Context

`docs/specs/2026-09-15-turnstile-and-queues.md` scoped AI and deliberately did
not build it, naming it the highest-design-risk item of that round. The
roadmap's **AI** item carried the founder's framing — *"some way to run AI on
the edge or connect to an internet API"* — with Workers AI explicitly
corrected as the wrong frame, and two questions left open before building:

1. whether `model` belongs in a declaration or per call;
2. whether a direct-vendor adapter is worth having as an escape hatch.

It also recorded the correction that settled the contract's shape: target an
**AI gateway**, not N model vendors.

The evidence that the N-vendor shape does not work is in the vendors' own
surfaces. Anthropic puts `system` at the top level; OpenAI makes it a message
role. Anthropic has content blocks; OpenAI has a string or a parts array.
Streaming event shapes differ substantially. And the surface moves — thinking
config, `tool_choice` and prefill all changed shape inside a year. Intersect
those by hand and the contract is too thin to write an agent against; superset
them and you have picked a vendor without admitting it, which is `MISSION.md`
principle 6 broken inside the file that implements principle 6.

## Decision

**The contract targets one shape — a gateway's — and `openrouter` is the first
implementation.** A gateway already performs the cross-vendor normalization
and, more to the point, maintains it. Vendor drift becomes the thing the
gateway is paid for.

**Model choice is a declaration: `[ai] model` in `fiducial.toml`**, derived
into the generated factory as a literal argument
(`ai: new OpenRouterAi(env, "anthropic/claude-opus-5")`). This is the answer to
open question 1, and it is what makes the gateway decision pay: switching from
Claude to GPT becomes a one-line change in a declaration with **no adapter
change at all**. A per-call `request.model` still wins over it — an agent that
classifies with a small model and answers with a large one is a normal agent,
and forcing it to construct a second adapter would make the declaration a lie
rather than a default.

No default model is supplied. Any default would be a vendor choice made on the
product's behalf, and would age into a model id that no longer exists —
silently, because a gateway reports an unknown model at the first call rather
than at deploy. `ai = "openrouter"` with no `[ai] model` fails `fid derive`,
naming the key.

**`NoneAi` fails rather than succeeding silently**, joining `NoneAuth` as the
second such `none`. The rule behind both, now that there are two: *does the
caller read a result?* A no-op send or enqueue is indistinguishable from the
real thing at the call site — the caller wanted an effect elsewhere. A session
and a completion **are** the result, so fabricating one relocates the failure
from the config that caused it to a blank answer in the UI.

**Answer to open question 2: no direct-vendor adapter.** `anthropic` and
`openai` are listed under `candidates`, which is this platform's word for
"intended, nothing behind it, cannot be selected."

## Why not the alternatives

**One adapter per model vendor.** The shape the roadmap sketched first, and
the reason it was called high-risk. Rejected on the evidence above: there is
no honest intersection, and the platform would have signed up to track four
vendors' breaking changes forever.

**The model as a wrangler `[vars]` entry rather than a factory argument.**
This was the shape that best preserved the uniform `new {Class}(env)` the
factory relies on. Rejected because `wrangler.toml` is generated only for
Cloudflare products, and this contract is reached from Next.js and SvelteKit
server routes too — the declaration would have derived for one deploy target
and been a hand-set environment variable everywhere else.

**The model per call only, with no declaration.** Simplest to implement and
it makes the model invisible to every gate: nothing would fail when a product
hardcoded a retired model id in four route handlers. The gateway decision is
only worth its indirection if the model is a fact declared in one place.

**Folding `ai` out of `AdapterSet` the way `auth` was.** `auth` was kept out
because a session store is *request-scoped* and cannot be built from `env` at
Worker scope. A declared model is not that — it is a constant `fid derive`
already knows, so writing it into the generated file is what every other
derived fact does. The slot stays env-scoped and stays in `AdapterSet`.

**`embed()` on the same contract.** Rejected for this round: turning product
data into vectors pairs with a vector store this platform does not have, so
the method would exist on every implementation with no task a product could
complete through it.

## Consequences

`fid derive --check` now **regenerates deterministic pipelines and compares**,
rather than only hashing outputs against `fiducial.lock`. `fid-schema` already
did this, and its doc comment named generalizing as "a change worth making when
a second one needs it." `fid-adapters` is that second one, and the generalization
closed a hole that predated AI: changing `database = "none"` to `"d1"` and
forgetting to re-run derive left a byte-identical file, a passing `--check`, and
a Worker still constructing `NoneDatabase`.

`@fiducial/adapters` gains a hand-written SSE parser. That is a real
maintenance cost, taken because the two ways SSE parsing goes wrong are both
about buffering — an event straddling a chunk boundary, and several events in
one chunk — and both fail only under load, which is when they are hardest to
see. Both are tested directly.

The Rust `Ai` trait needs a stream type, so `fiducial-adapters` takes a
dependency on `futures-core` — the `Stream` trait alone, no executor and no
combinators, so no consumer inherits a runtime from the contract.

## What this deliberately does not do

- **No embeddings, and no vector store.** Named as a real second use with its
  own shape; it lands when it has a consumer.
- **No direct-vendor adapters.** Adding one later is adding an implementation,
  not redesigning the contract — which is the property that makes deferring it
  cheap.
- **No Rust implementation of `openrouter`.** Same boundary as `turnstile` and
  `supabase` (auth): a plain HTTPS API Rust could reach, with no Rust consumer
  that would. The trait exists so the contract is one contract in both
  languages.
- **No agent loop, no retry policy, no prompt templating.** The contract is the
  vendor boundary. What to do with a `tool_calls` stop reason is the product's
  decision, and a platform that answered it here would be shipping an opinion
  about agents rather than an adapter.
