---
"@fiducial/board-schema": minor
---

Add `connector.mount` and enclosure standoff options to the board schema.

A connector may now declare where it meets the case wall, and `fid derive`
punches an opening for it. `mount` carries `side` and `offset_mm` in board
coordinates plus optional `width_mm` / `height_mm` / `z_offset_mm`; sizes
default to the connector family's body envelope, newly exported as
`CONNECTOR_OPENINGS` and resolved by `mountEnvelope(mount, type)`.

`mount` is optional per connector, and its absence means no hole — the safe
default in a sealed case, and the right answer for a header reached with the
lid off.

`outline.enclosure` gains `standoff_height_mm` and `standoff_size_mm`, which
lift the board onto four corner posts.

`parseBoardInterface()` validates all of it, mirroring `fiducial_eda::validate`:
an unknown side, a negative offset, a non-positive size, a mount with no
outline to measure against, and a connector family whose size cannot be
resolved are all rejected.
