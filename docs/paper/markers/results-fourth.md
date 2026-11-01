# Injected mistakes, fourth set — fid 0.10.0 (6b164ef)

Markers caught: **14 / 20**. Controls passed: **0 / 0**.

| ID | Area | Class | Mistake | Caught by |
|---|---|---|---|---|
| F1 | brand | declaration bypassed | A hand-written site footer types the legal name, contact email, domain and primary colour as literals instead of importing src/generated/brand.ts. | --check |
| F2 | firmware | declaration bypassed | The firmware's LED colour matches on hand-typed humidity counts (3500..=6000) instead of the generated comfortable() band from protocol.toml. | --check |
| F3 | web | declaration bypassed | The page narrows its Web Serial filter with the USB product ID typed as a literal 0x000a, beside the imported vendor ID, so replacing the placeholder IDs in protocol.toml leaves the page unable to find the stick. | --check |
| F4 | ci | declaration bypassed | The derive-check step in the web product's CI is marked continue-on-error, so a stale artifact shows a warning and the build stays green. | --check |
| F5 | ci | declaration bypassed | The CI step is changed from `fid derive --check` to plain `fid derive`, which regenerates everything on the runner and can never fail on staleness. | --check |
| F6 | ci | declaration bypassed | In the root CI's sensor-stick job the derive-check step is moved after hardware/build.sh, which runs `fid derive --pipeline hardware` itself, so stale hardware artifacts are regenerated before they are checked. | **not caught** |
| F7 | hardware | copied, not translated | The AHT20's I²C address is changed to 0x70, the 8-bit write address in the datasheet's table, where the field holds the 7-bit address 0x38. | derive, --check |
| F8 | protocol | declaration breaks a rule | The humidity field's `scale` key is misspelled `sacle`, so the reading and the comfortable band lose their ×0.01 scale. | derive, --check |
| F9 | brand | copied, not translated | [brand] domain is given with its scheme ("https://example.com"), where every derivation prefixes https:// itself. | derive |
| F10 | hardware | declaration breaks a rule | The I²C pull-ups are changed from 4.7 kΩ to 470 Ω for faster edges: 3.3 V / 470 Ω ≈ 7 mA, above the 3 mA sink current the I²C standard allows (LCSC part left unchanged). | derive, --check |
| F11 | hardware | declaration breaks a rule | Track width and clearance are cut to 0.075 mm (3 mil) to make the QFN escape easier, below what the declared fab's standard two-layer process can make. | derive, --check |
| F12 | legal | declaration breaks a rule | Analytics and marketing cookie categories are declared, and the banner initialises its checkboxes with every optional category selected: pre-ticked consent, invalid under GDPR/ePrivacy. | derive, --check |
| F13 | seo | incomplete declaration | scripts/derive-seo.mjs is edited to merge extra pages from a new apps/web/src/app/extra-pages.json into the sitemap; the file is not among the seo pipeline's declared inputs. | derive |
| F14 | content | incomplete declaration | scripts/derive-content.mjs is edited to give every entry an author from [brand] trading_name, but the content pipeline declares only fiducial.toml#i18n, so a later brand rename leaves content.ts stale without the gate noticing. | **not caught** |
| F15 | hardware | incomplete declaration | hardware/cad.py is edited to overlay a new hardware/case-tweaks.json on the solved layout (1.0 mm walls, 0.1 mm clearance), an input nothing declares that overrides product.toml's [case]. | **not caught** |
| F16 | firmware | declarations conflict | memory.x is changed to a 16 MB flash region (copied from a W25Q128 board) while the declared flash part is a 32 Mbit (4 MB) W25Q32, so a large image links and then cannot fit on the chip. | **not caught** |
| F17 | design | copied, not translated | The muted-foreground grey is lightened, and the two contrast pairs that then fail are relabelled `large-text` with a 3.0 minimum, although that colour is used for captions and small secondary text. | **not caught** |
| F18 | i18n | declarations conflict | A hand-written Next.js middleware keeps its own locale list and redirects to a default of "en", while [i18n] declares the default as "sr". | **not caught** |
| F19 | hardware | declaration breaks a rule | The LED level-shifting buffer's supply pin is moved from VBUS to 3V3, so its output tops out at 3.3 V, below the WS2812B's 3.5 V input-high threshold at 5 V; its decoupling cap stays on VBUS. | derive, --check |
| F20 | migrations | declarations conflict | A hand-written migrations/0002_profiles.sql is added, then [identity] audit = true is turned on, which derives migrations/0002_audit.sql: two migrations share number 0002. | derive, --check |
