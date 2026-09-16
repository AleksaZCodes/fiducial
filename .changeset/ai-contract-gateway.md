---
"@fiducial/adapters": minor
---

Add the `ai` contract — conversational and agentic language-model calls —
with `openrouter` as its first vendor.

The contract targets an **AI gateway**, not one adapter per model vendor.
There is no honest intersection of the Anthropic and OpenAI shapes: `system`
is top-level in one and a message role in the other, content blocks meet a
parts array, streaming events differ substantially, and three parameters
changed shape within a year. Intersecting those by hand yields a contract too
thin to write an agent against; supersetting them picks a vendor without
admitting it.

- `Ai` — `chat()` for a completed response, `stream()` for an
  `AsyncIterable<StreamEvent>`. Messages, a top-level system prompt, tool
  definitions, tool calls, tool results, stop reasons and usage.
- `OpenRouterAi` — secret-reached (`env.OPENROUTER_API_KEY`), so it works
  from any runtime with `fetch`.
- `NoneAi` **throws** rather than returning an empty completion, joining
  `NoneAuth`. The rule, now that there are two: a no-op send is
  indistinguishable from a real one at the call site, but a completion *is*
  the result — fabricating one turns "no vendor selected" into a blank answer
  in the UI, debugged as a model bug.

Streamed tool calls are emitted whole. Gateways stream tool arguments as
partial JSON, which a consumer can do nothing with but buffer, so the adapter
buffers once.

`AdapterSet` gains an `ai` field, and `createNoneAdapters()` fills it.

Select with `ai = "openrouter"` in `[adapters]` and declare `[ai] model` —
`fid derive` passes the model into the generated factory and names
`OPENROUTER_API_KEY` in the derived `wrangler.toml`. A per-call
`request.model` overrides the declared default.
