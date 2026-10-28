# Injected mistakes, independent set, without Fiducial (ordinary CI only) — fid 0.9.12 (16f6d77)

Markers caught: **0 / 20**. Controls passed: **0 / 0**.

| ID | Area | Class | Mistake | Caught by |
|---|---|---|---|---|
| I1 | i18n | incomplete declaration | German added to [i18n] locales with no messages/de.json | **not caught** |
| I2 | i18n | copied, not translated | Serbian catalog's locale.tag copied from English (en-GB) | **not caught** |
| I3 | content | declarations conflict | author field added to both posts' frontmatter, not to the schema | **not caught** |
| I4 | content | stale output | Blog route changed to /posts/{slug}, not re-derived | **not caught** |
| I5 | SEO | derived file edited | A /pricing URL added to the generated sitemap.xml by hand | **not caught** |
| I6 | brand | declaration bypassed | New footer component hardcodes the origin, legal name and contact email | **not caught** |
| I7 | legal | declaration breaks a rule | cookie_categories set to analytics only, dropping necessary | **not caught** |
| I8 | design | declaration breaks a rule | Chip chamfer set to 2rem, more than half a chip's height | **not caught** |
| I9 | database | declarations conflict | New migration written in Postgres syntax (SERIAL, TIMESTAMPTZ, now()) for a SQLite product | **not caught** |
| I10 | deployment | declaration bypassed | CI step running fid derive --check deleted to get a red build green | **not caught** |
| I11 | hardware | declarations conflict | Regulator swapped for the 5.0 V variant of the AMS1117 while its output is still the 3V3 rail | **not caught** |
| I12 | circuit | declarations conflict | LED data input wired straight to the 3.3 V GPIO net, skipping the level-shifting buffer | **not caught** |
| I13 | hardware | incomplete declaration | Decoupling caps placed near mcu.VDD, a pin the RP2040 does not have | **not caught** |
| I14 | mechanical | declarations conflict | Outline drawing shortened to 30x40, too short for the 79 mm board | **not caught** |
| I15 | mechanical | incomplete declaration | through_wall removed from the USB-C socket, so the case has no plug window | **not caught** |
| I16 | protocol | declarations conflict | New status message declared with kind 1, the reading's kind | **not caught** |
| I17 | protocol | stale output | USB product ID changed in protocol.toml, not re-derived | **not caught** |
| I18 | firmware | declaration bypassed | Firmware enumerates with literal USB IDs instead of the generated constants | **not caught** |
| I19 | hardware | derived file edited | Generated BOM edited by hand to order a sixth decoupling cap | **not caught** |
| I20 | web | declaration bypassed | Page scales the reading by a hand-typed /100 instead of the declared scale | **not caught** |
