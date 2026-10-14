---
description: A Fiducial product's state in one step — `fid doctor`'s verdict, installed capabilities and guard rules. Use once, when starting work in a directory that contains fiducial.toml. Not for the Fiducial platform repo itself, and not before a product exists (use `fid new`).
---

## `fid doctor`

!`fid doctor 2>&1 | tail -n 40`

## `fiducial.toml`

!`cat fiducial.toml 2>/dev/null || echo "No fiducial.toml here — this is not a Fiducial product. Stop: use \`fid new <name>\` to make one."`

---

Fix anything doctor reports as an issue before other work; upgrade hints can
wait for `fid upgrade`.

The product's rules — commands, guard, propagation, what not to do — are in
`AGENTS.md`. Read it once if it is not already in your context, and not again
on later turns. `MISSION.md` is the tiebreaker: open it when a decision is
genuinely ambiguous, not by default.
