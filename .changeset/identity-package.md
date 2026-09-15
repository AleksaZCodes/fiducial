---
"@fiducial/identity": minor
---

New package: `@fiducial/identity` — one identity model across users, devices
and services.

`Principal` spans `user`, `device`, `service` and `anonymous`;
`can(principal, action, resource, grants)` is the whole authorization rule.
Deny by default, an unidentified principal (anonymous, or an all-zero
sentinel id) is refused, and a device may read itself without a grant but
never write itself.

Mirrors the Rust `fiducial-identity` crate, which firmware, the desktop
backend and the CLI run. Both are held to `docs/identity/vectors.json` — a
decision table generated from the Rust crate and replayed here — because an
authorization divergence does not look like a bug, it looks like access.

`principalFromSession()` bridges from the `auth` adapter contract: an absent
or unverified session becomes `ANONYMOUS`, which `can()` refuses.
