# Injected mistakes, independent set — fid 0.9.12 (16f6d77)

Markers caught: **16 / 20**. Controls passed: **0 / 0**.

| ID | Area | Class | Mistake | Caught by |
|---|---|---|---|---|
| I1 | i18n | incomplete declaration | German added to [i18n] locales with no messages/de.json | derive |
| I2 | i18n | copied, not translated | Serbian catalog's locale.tag copied from English (en-GB) | derive, --check |
| I3 | content | declarations conflict | author field added to both posts' frontmatter, not to the schema | derive |
| I4 | content | stale output | Blog route changed to /posts/{slug}, not re-derived | --check |
| I5 | SEO | derived file edited | A /pricing URL added to the generated sitemap.xml by hand | --check |
| I6 | brand | declaration bypassed | New footer component hardcodes the origin, legal name and contact email | **not caught** |
| I7 | legal | declaration breaks a rule | cookie_categories set to analytics only, dropping necessary | **not caught** |
| I8 | design | declaration breaks a rule | Chip chamfer set to 2rem, more than half a chip's height | derive, --check |
| I9 | database | declarations conflict | New migration written in Postgres syntax (SERIAL, TIMESTAMPTZ, now()) for a SQLite product | derive, --check |
| I10 | deployment | declaration bypassed | CI step running fid derive --check deleted to get a red build green | **not caught** |
| I11 | hardware | declarations conflict | Regulator swapped for the 5.0 V variant of the AMS1117 while its output is still the 3V3 rail | derive, --check |
| I12 | circuit | declarations conflict | LED data input wired straight to the 3.3 V GPIO net, skipping the level-shifting buffer | derive, --check |
| I13 | hardware | incomplete declaration | Decoupling caps placed near mcu.VDD, a pin the RP2040 does not have | derive, --check |
| I14 | mechanical | declarations conflict | Outline drawing shortened to 30x40, too short for the 79 mm board | derive, --check |
| I15 | mechanical | incomplete declaration | through_wall removed from the USB-C socket, so the case has no plug window | type check |
| I16 | protocol | declarations conflict | New status message declared with kind 1, the reading's kind | derive, --check |
| I17 | protocol | stale output | USB product ID changed in protocol.toml, not re-derived | --check |
| I18 | firmware | declaration bypassed | Firmware enumerates with literal USB IDs instead of the generated constants | derive, --check |
| I19 | hardware | derived file edited | Generated BOM edited by hand to order a sixth decoupling cap | --check |
| I20 | web | declaration bypassed | Page scales the reading by a hand-typed /100 instead of the declared scale | **not caught** |
