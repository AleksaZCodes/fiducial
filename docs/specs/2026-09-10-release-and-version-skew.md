# Release tooling and wire-protocol version skew

_Date: 2026-09-10 | Phase 16b_

## Problem

A device running old firmware speaks one wire protocol version.  The desktop
app gets upgraded and speaks a new version.  Without explicit enforcement, the
two silently misparse each other's frames — or, worse, corrupt state without
error.

## Decisions

### One wire-version constant, one committed matrix

`fiducial_protocol::WIRE_VERSION: u8` is the single declaration of the wire
protocol version in the Rust workspace.  Every artifact that links
`fiducial-protocol` carries this constant at compile time.

`docs/compat/matrix.toml` is the committed policy file:

```toml
[wire]
current        = 1   # what this build speaks
min_compatible = 1   # oldest remote version we accept
```

`fid release check` fails CI when these disagree.  A PR that bumps one
without the other cannot merge.

### Breaking vs. compatible bumps

Two bump kinds, one command:

| Kind | Effect | Old artifact |
|---|---|---|
| `--bump breaking` | current+1, min_compatible=current | **rejected** at connect |
| `--bump compatible` | current+1, min_compatible unchanged | accepted |

Default policy: every wire-format change is a breaking bump unless explicitly
declared compatible.

### `assert_compatible` is the enforcement point

```rust
// In the connection handshake:
assert_compatible(WIRE_VERSION, remote_version, matrix.min_compatible)?;
```

Returns `Err(VersionSkewError)` when `remote < min_compatible`.  The error
carries all three fields so the operator knows exactly which artifact to
upgrade.

### `fid release` owns the matrix

`docs/compat/matrix.toml` is written by `fid release protocol --bump` only.
The rule mirrors the platform-wide "one declaration, many derivations" principle:
the command is the declaration, the file is the derivation, `fid release check`
asserts they agree.

## What is not decided here

- Automatic migration paths for devices on incompatible versions (Phase 16c OTA)
- The handshake frame format itself (currently application-layer; will move to
  the framing layer when a second breaking change is needed)
- Multi-version server support — a host that bridges devices on different wire
  versions simultaneously.  Not yet needed; left for when it is.
