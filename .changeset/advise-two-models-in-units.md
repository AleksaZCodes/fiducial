---
"@fiducial/advisor": minor
---

`fid advise` reviews a change with two models, in structural units. The diff is split into units (a file, the hunks of a large file, or a directory of small ones) and each is reviewed on its own, so a large change costs more requests instead of a truncated review. System One still answers the typed questions, one request per unit, and the results are merged. A reasoning model (`anthropic/claude-haiku-5.5` through OpenRouter, `FIDUCIAL_REVIEW_MODEL` to change it) reads each unit against the product's own `AGENTS.md` and `MISSION.md`, with a manifest of the repository's files, and one more request looks across the whole change for documentation it left stale. Derived outputs, lockfiles and unmodified platform files are skipped by name. The report ends with what the calling agent should do with it, and `--json` carries the same as data. `--no-review` asks System One only.
