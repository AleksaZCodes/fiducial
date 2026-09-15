# A device proves it is that device

**Date:** 2026-09-15
**Status:** implemented

---

## Context

`can()` answers *may they*. It assumes the caller already knows **who is
asking**, and for a user the `auth` adapter supplies that. For a device or a
service there was nothing. `docs/specs/2026-09-15-identity-at-every-level.md`
named the gap and stopped:

> no device/service token issuance — how a device *proves* it is that device
> needs its own pass with real cryptographic choices.

Without it, `Principal::Device(id)` is an assertion anyone can make. The whole
identity model rests on a claim nothing checks.

## Decision

A fixed-layout, domain-separated ed25519 bearer token: 41 bytes of claims, 64
bytes of signature, 105 total.

### Ed25519

Already in this workspace for signed OTA manifests, already understood by the
firmware that has to verify offline, small keys and small signatures. And
**deterministic** — no per-signature entropy, which matters on a
microcontroller where the RNG is the least trustworthy peripheral on the die.

### Not JWT

A JWT is JSON, base64, and a header that names its own algorithm. The header is
the problem: `alg: none` and algorithm-confusion attacks exist because the
token gets to say how it should be checked. Here the layout is fixed, the
algorithm is not in the token, and the verifier decides.

It is also 105 bytes rather than several hundred, which on a constrained radio
link is the difference between fitting in a frame and not.

### A domain separator, signed

A token and an OTA manifest are both ed25519 signatures by the platform key
over some bytes. Without a separator, a blob valid as one could be presented as
the other and the verifier would be right to accept it. `signing_input` prefixes
`fiducial-identity-token-v1`, and a test signs bare claims with the same key and
asserts the result does **not** verify.

### Expiry is mandatory

A [`Grant`](../../crates/fiducial-identity/src/lib.rs) may legitimately be
permanent — that is what an owner holds. A *bearer credential* that never
expires is a password that cannot be changed, and `issue()` refuses claims
whose expiry is at or before their issue time.

Verification **fails closed without a clock**, the same rule grant expiry
follows: a verifier that cannot tell the time cannot honour an expiry, and a
bearer credential whose expiry is unenforceable is a permanent one.

### Signature before parse

`verify` checks the signature before decoding the claims, even though decoding
first would allow a more specific error. Reading structure out of an unverified
token and acting on it is how a parser becomes the attack surface.

### `verify_as` exists because forgetting the binding is silent

`verify` says the token is authentic. It does not say it is *yours*. A caller
that checks only the signature accepts any valid token as any principal — the
cryptography is perfect and the binding to who is asking is simply missing.
`verify_as` is the call almost everyone wants, and it is a separate function so
that using the weaker one is a choice.

## Consequences

`no_std` on every device target, with and without software crypto, checked in
CI: a device with a hardware accelerator implements `SignatureVerifier` against
that and never links a software implementation it will not call — the split
`fiducial-ota` already makes.

**The format is pinned by conformance vectors**, generated from the Rust crate
into `docs/identity/token-vectors.json` and replayed by the TypeScript verifier.
A divergence in a token format does not look like a bug: it looks like an
outage on one side or a bypass on the other. Verified adversarially — changing
one character of the domain separator in TypeScript alone fails 23 of 33
checks. The vectors cover both expiry boundaries, because *valid until* is the
half of an interval two implementations disagree about.

**TypeScript verifies and cannot issue.** Issuing needs a private key, and a
Worker is not where one lives: a signing key in edge code is a signing key in
every deploy log and every error report.

## What this deliberately does not do

- **No revocation list.** A token is short-lived and that is its revocation
  story: the window is the lifetime. A revocation list is a lookup on every
  verification, which defeats offline verification — the property this exists
  for. Shorten the lifetime instead.
- **No refresh flow.** Re-issuing is the refresh flow. A refresh token is a
  second, longer-lived credential, which is the thing this was careful not to
  create.
- **No key rotation policy.** The verifier is handed a key; which key, and when
  it changes, is a deployment decision this crate should not make for anyone.
- **No nonce replay cache.** The nonce makes two tokens distinguishable, so a
  log can tell a replay from a fresh issue. Actually *refusing* a replay needs
  shared state with a retention policy, which is a service, not a type.
- **No provisioning.** How a device's key gets onto the device and its public
  half into the platform is a manufacturing question with real physical
  constraints, and guessing at it here would produce a contract shaped for no
  factory.
