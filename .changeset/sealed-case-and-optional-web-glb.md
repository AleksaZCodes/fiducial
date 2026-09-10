---
"@fiducial/board-schema": minor
---

Add `outline.enclosure` to the `BoardInterface` schema — optional overrides for the generated case (`headroom_mm`, `lid_thickness_mm`, `gasket_width_mm`, `gasket_height_mm`, `gasket_compression`). `parseBoardInterface()` rejects non-positive values and a `gasket_compression` outside the exclusive range (0, 1), mirroring `fiducial_eda::validate`.

These are the values a manufacturing process cannot imply. Clearance, lip width, floor thickness, and gasket fit remain derived from the board's tolerance class.
