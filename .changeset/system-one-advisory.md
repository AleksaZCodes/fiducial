---
"@fiducial/adapters": minor
"@fiducial/advisor": minor
---

A second kind of model, and an advisory layer that never gates.

`SystemOne` is a new adapter contract for models that answer typed questions
about a state and return calibrated probability distributions rather than text.
TypeSafe's Jev is the first. It is a peer of `Ai`, not a method on it: `Ai` takes
a turn history and returns prose or tool calls, and coercing typed decisions
through that shape is the exact mismatch these models exist to remove.

Two vendors, one wire shape. `OpenRouterSystemOne` posts to
`/api/alpha/decisions` — not under `/api/v1`, not chat completions — which
proxies to TypeSafe through a dedicated decisions router, so the typed-answer
shape survives the gateway. `TypeSafeSystemOne` goes direct. They differ only in
URL, secret, and whether the model id carries a `typesafe/` prefix. `openrouter`
is the default because a product declaring `ai = "openrouter"` already holds the
key, and one secret to rotate beats two.

Three question types with differently-shaped answers: Noul (yes/no, returns a
probability and deliberately no `confidence` — the value already is the
distribution), Choice (one of N, with a distribution over all options), Score
(a position on ordered levels, which can land between them). `noulOf`,
`choiceOf` and `scoreOf` narrow an answer by key so a mixed-up question id is a
caught error rather than a silent cast. Both vendors share one retry loop with
exponential backoff, jitter, and `Retry-After`, because both APIs require
backoff on 429 and 529 and neither ships that for free without its SDK.

`@fiducial/advisor` is new: `fid-advise`, reached as `fid advise`. It reviews a
diff for the rules an exact check cannot reach — a fact declared twice under two
names, logic sitting above the layer it could reach, a comment explaining what
instead of why. Rules that *are* decided exactly are deliberately not asked.
Every check is a hybrid: code narrows candidates deterministically and the model
judges only what survives.

**`AdapterSet` gains a `systemOne` member.** Code that builds an `AdapterSet` by
hand needs one more field; `createNoneAdapters()` and the factory `fid derive`
generates are already updated.
