---
"@fiducial/board-schema": minor
---

Add `outline.enclosure.fastener_diameter_mm` to the board schema.

Declaring a shaft diameter gives the generated case four corner screws that
retain the lid — a clearance hole through the lid and a pilot hole in the base.
Without them nothing clamps the lid, so the gasket only compresses while
something external holds it shut.

The screws pass through the outer lip, outboard of the gasket groove: a hole
inside the gasket line would open the sealed cavity. That lip has to carry the
hole with a printable wall either side, so declaring a fastener thickens the
case wall — which is why it is opt-in rather than assumed.

`parseBoardInterface()` rejects a non-positive diameter, mirroring
`fiducial_eda::validate`.
