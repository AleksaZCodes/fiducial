# Injected mistakes, first set, without Fiducial (ordinary CI only) — fid 0.9.11 (bbaa1b0)

Markers caught: **1 / 20**. Controls passed: **5 / 5**.

| ID | Area | Class | Mistake | Caught by |
|---|---|---|---|---|
| W1 | i18n | incomplete declaration | UI string added in English only | **not caught** |
| W2 | i18n | stale output | English string edited, not re-derived | **not caught** |
| W3 | i18n | derived file edited | Derived message module edited by hand | **not caught** |
| W4 | content | incomplete declaration | Post published in English only | **not caught** |
| W5 | SEO | incomplete declaration | Page declared without title or description | **not caught** |
| W6 | brand | stale output | Domain changed, not re-derived | **not caught** |
| W7 | deploy | derived file edited | Derived Worker config edited by hand | **not caught** |
| W8 | design | declaration breaks a rule | Secondary text below 4.5 : 1 contrast | **not caught** |
| W9 | database | derived file edited | Applied migration edited in place | **not caught** |
| W10 | i18n | copied, not translated | Serbian string left in English | **not caught** |
| H1 | hardware | stale output | Sensor pin moved, not re-derived | **not caught** |
| H2 | firmware | derived file edited | Derived pin map edited by hand | **not caught** |
| H3 | circuit | declaration breaks a rule | USB-C pull-down 5.1 kΩ → 22 kΩ | **not caught** |
| H4 | placement | declarations conflict | Sensor zone contradicts its vent | **not caught** |
| H5 | cost | declaration breaks a rule | Cost ceiling lowered to USD 3 | **not caught** |
| H6 | interface | consumer out of date | Net renamed, code not updated | **not caught** |
| H7 | firmware | declaration bypassed | Pin named directly, then moved | **not caught** |
| H8 | protocol | derived file edited | Message layout changed in firmware only | firmware tests |
| H9 | firmware | declaration bypassed | Sensor I²C address typed (and mistyped) | **not caught** |
| H10 | mechanical | declarations conflict | Case narrowed 36 → 28 mm | **not caught** |
| C1 | hardware | correct change | Sensor pin moved to GPIO8, re-derived | passes every gate |
| C2 | i18n | correct change | UI string added in both languages | passes every gate |
| C3 | protocol | correct change | Comfortable band widened to 65 %RH | passes every gate |
| C4 | - | no change | Web product re-derived unchanged | passes every gate |
| C5 | - | no change | Sensor stick re-derived unchanged | passes every gate |
