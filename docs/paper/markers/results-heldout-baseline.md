# Injected mistakes, held-out set, without Fiducial (ordinary CI only) — fid 0.9.11 (bbaa1b0)

Markers caught: **0 / 20**. Controls passed: **0 / 0**.

| ID | Area | Class | Mistake | Caught by |
|---|---|---|---|---|
| N1 | i18n | declaration breaks a rule | Placeholder renamed in Serbian only | **not caught** |
| N2 | i18n | incomplete declaration | String deleted from Serbian only | **not caught** |
| N3 | legal | declaration breaks a rule | Jurisdiction mistyped | **not caught** |
| N4 | brand | declaration breaks a rule | Brand colour mistyped (five hex digits) | **not caught** |
| N5 | SEO | declarations conflict | Same page declared twice | **not caught** |
| N6 | content | declaration breaks a rule | Post dated a month that does not exist | **not caught** |
| N7 | content | incomplete declaration | Serbian post missing its title | **not caught** |
| N8 | design | declaration breaks a rule | Contrast pair names a colour that does not exist | **not caught** |
| N9 | database | declaration breaks a rule | New migration uses a type the database lacks | **not caught** |
| N10 | identity | stale output | Permissions table renamed, not re-derived | **not caught** |
| N11 | hardware | incomplete declaration | LED data net mistyped on the LED | **not caught** |
| N12 | hardware | declarations conflict | Two parts given the same id | **not caught** |
| N13 | circuit | incomplete declaration | I²C pull-up resistor deleted | **not caught** |
| N14 | parts | declaration breaks a rule | Wrong order number on a resistor | **not caught** |
| N15 | mechanical | declaration breaks a rule | Case wall thinned 1.6 → 0.4 mm | **not caught** |
| N16 | mechanical | declarations conflict | Board thickness 1.6 → 3.0 mm | **not caught** |
| N17 | protocol | copied, not translated | Temperature declared unsigned | **not caught** |
| N18 | protocol | declaration breaks a rule | Band bounds swapped (min above max) | **not caught** |
| N19 | firmware | declaration bypassed | Sensor wait typed as a number | **not caught** |
| N20 | web | declaration bypassed | Page compares humidity to the band by hand | **not caught** |
