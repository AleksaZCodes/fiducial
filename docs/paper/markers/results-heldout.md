# Injected mistakes, held-out set — fid 0.9.11 (bbaa1b0)

Markers caught: **15 / 20**. Controls passed: **0 / 0**.

| ID | Area | Class | Mistake | Caught by |
|---|---|---|---|---|
| N1 | i18n | declaration breaks a rule | Placeholder renamed in Serbian only | derive |
| N2 | i18n | incomplete declaration | String deleted from Serbian only | derive |
| N3 | legal | declaration breaks a rule | Jurisdiction mistyped | derive, --check |
| N4 | brand | declaration breaks a rule | Brand colour mistyped (five hex digits) | derive |
| N5 | SEO | declarations conflict | Same page declared twice | **not caught** |
| N6 | content | declaration breaks a rule | Post dated a month that does not exist | **not caught** |
| N7 | content | incomplete declaration | Serbian post missing its title | derive |
| N8 | design | declaration breaks a rule | Contrast pair names a colour that does not exist | derive, --check |
| N9 | database | declaration breaks a rule | New migration uses a type the database lacks | derive, --check |
| N10 | identity | stale output | Permissions table renamed, not re-derived | --check |
| N11 | hardware | incomplete declaration | LED data net mistyped on the LED | derive, --check |
| N12 | hardware | declarations conflict | Two parts given the same id | derive, --check |
| N13 | circuit | incomplete declaration | I²C pull-up resistor deleted | derive, --check |
| N14 | parts | declaration breaks a rule | Wrong order number on a resistor | derive, --check |
| N15 | mechanical | declaration breaks a rule | Case wall thinned 1.6 → 0.4 mm | derive, --check |
| N16 | mechanical | declarations conflict | Board thickness 1.6 → 3.0 mm | **not caught** |
| N17 | protocol | copied, not translated | Temperature declared unsigned | firmware build, firmware tests, web test |
| N18 | protocol | declaration breaks a rule | Band bounds swapped (min above max) | derive, --check |
| N19 | firmware | declaration bypassed | Sensor wait typed as a number | **not caught** |
| N20 | web | declaration bypassed | Page compares humidity to the band by hand | **not caught** |
