---
"@fiducial/adapters": patch
"@fiducial/advisor": patch
---

The OpenRouter default decisions model is `~typesafe/jev-latest`. OpenRouter publishes "latest" aliases with a leading `~`, so the previous default, `typesafe/jev-latest`, was rejected with HTTP 400 and `fid advise` skipped every run with the default configuration.
