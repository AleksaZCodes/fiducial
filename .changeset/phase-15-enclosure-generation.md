---
"@fiducial/board-schema": minor
---

Add `outline` to the `BoardInterface` schema — `width_mm`, `height_mm`, optional `thickness_mm` (default 1.6) and `tolerance` (`fdm` | `resin` | `cnc`). `parseBoardInterface()` now rejects non-positive dimensions and unknown tolerance classes, mirroring `fiducial_eda::validate`. When declared, `fid derive` generates the enclosure STL and GLB from this block.
