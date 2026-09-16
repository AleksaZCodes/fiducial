# Security Policy

## Reporting a vulnerability

**Do not open a public issue.** A public report is a disclosure, and it is
also prior art — see [`IP-POLICY.md`](IP-POLICY.md).

Two channels, in order of preference:

1. **GitHub private vulnerability reporting** — the "Report a vulnerability"
   button under this repository's *Security* tab. Preferred: it gives the
   report a private thread, a tracked advisory, and a path to a CVE if one is
   warranted.
2. **Email** — <aleksazdravkovic@proton.me>, for anyone without a GitHub
   account or where the button is unavailable.

Please include what it affects, how to reproduce it, and what an attacker
gets. A proof of concept is welcome and never required.

### What to expect

This project is maintained by one person. The honest commitment is an
acknowledgement within **7 days** and an assessment within **30**. If you have
not heard back in 7 days, assume the message was lost rather than ignored, and
send it again by the other channel.

You will be credited in the advisory unless you ask not to be. There is no
bounty programme.

## What is in scope

| In scope | Not in scope |
|---|---|
| The `fid` CLI, including `fid guard-check` | Findings in a dependency, unless this repository's use of it is what makes it exploitable — report those upstream |
| The published crates and npm packages | Anything requiring an attacker to already control the machine running `fid` |
| Generated artifacts that carry a security property — the identity grants schema and its RLS policies, the OTA manifest signature path, the protocol framing and checksums | A scaffolded product's own code. `fid` generates a starting point; what a product does afterwards is the product's |
| Adapter implementations that handle credentials or verify tokens | Missing hardening in a `none` adapter. `none` is a documented no-op, and `botProtection = "none"` failing open is [specified behaviour](docs/specs/2026-09-15-turnstile-and-queues.md), not a vulnerability |

A **secret committed to this repository** is always in scope and always
urgent, whoever committed it.

## Supported versions

No tagged release exists yet, so there is nothing to backport to: the
supported version is the current `main`. This section becomes real when
releases do, and the release work is what should replace this paragraph.
