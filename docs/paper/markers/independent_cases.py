# The independent set: 20 mistakes written on 2026-10-28 by a separate agent
# that read only the two products' files (not the platform's rules, specs or
# history) and committed here before their first run, on 0.9.12. One breach,
# reported by that agent: a read of run.py showed it the first five cases of
# the original set (W1-W5); none of these repeats them. Loaded by run.py.

INDEPENDENT = [
    # ── web product ──
    Case("I1", W, "i18n", INCOMPLETE, "German added to [i18n] locales with no messages/de.json",
         lambda r: edit(r / "fiducial.toml", '    "sr",\n]\ndefault', '    "sr",\n    "de",\n]\ndefault'), True),
    Case("I2", W, "i18n", MEANING, "Serbian catalog's locale.tag copied from English (en-GB)",
         lambda r: json_set(r / "messages/sr.json", "locale.tag", "en-GB"), True),
    Case("I3", W, "content", CONFLICT, "author field added to both posts' frontmatter, not to the schema",
         lambda r: (edit(r / "content/posts/en/hello.md", "tags: [example]\n", "tags: [example]\nauthor: Aleksa\n"),
                    edit(r / "content/posts/sr/hello.md", "tags: [example]\n", "tags: [example]\nauthor: Aleksa\n")), True),
    Case("I4", W, "content", STALE, "Blog route changed to /posts/{slug}, not re-derived",
         lambda r: edit(r / "content.toml", 'route = "/blog/{slug}"', 'route = "/posts/{slug}"'), False),
    Case("I5", W, "SEO", HAND, "A /pricing URL added to the generated sitemap.xml by hand",
         lambda r: edit(r / "apps/web/public/sitemap.xml", "</urlset>",
                        "  <url>\n    <loc>https://example.com/pricing</loc>\n  </url>\n</urlset>"), False),
    Case("I6", W, "brand", BYPASS, "New footer component hardcodes the origin, legal name and contact email",
         lambda r: (r / "apps/web/src/components/site-footer.tsx").write_text(
             'export function SiteFooter() {\n'
             '  return (\n'
             '    <footer className="type-small">\n'
             '      <a href="https://example.com/privacy">Privacy</a> · © 2026 Example LLC ·{" "}\n'
             '      <a href="mailto:hello@example.com">hello@example.com</a>\n'
             '    </footer>\n'
             '  );\n'
             '}\n'), True),
    Case("I7", W, "legal", RULE, "cookie_categories set to analytics only, dropping necessary",
         lambda r: edit(r / "fiducial.toml", 'cookie_categories = ["necessary"]', 'cookie_categories = ["analytics"]'), True),
    Case("I8", W, "design", RULE, "Chip chamfer set to 2rem, more than half a chip's height",
         lambda r: edit(r / "design-system.md", 'chip          = "0.375rem"', 'chip          = "2rem"'), True),
    Case("I9", W, "database", CONFLICT, "New migration written in Postgres syntax (SERIAL, TIMESTAMPTZ, now()) for a SQLite product",
         lambda r: (r / "migrations/0002_grant_audit.sql").write_text(
             "-- Audit trail for grant changes.\n"
             "CREATE TABLE IF NOT EXISTS grant_audit (\n"
             "  id           SERIAL PRIMARY KEY,\n"
             "  principal_id TEXT NOT NULL,\n"
             "  action       TEXT NOT NULL,\n"
             "  at           TIMESTAMPTZ NOT NULL DEFAULT now()\n"
             ");\n"), True),
    Case("I10", W, "deployment", BYPASS, "CI step running fid derive --check deleted to get a red build green",
         lambda r: edit(r / ".github/workflows/ci.yml",
                        "\n      # Fails when any derived artifact no longer matches its declaration.\n"
                        "      - name: fid derive --check\n        run: fid derive --check\n", "\n"), True),

    # ── sensor stick ──
    Case("I11", S, "hardware", CONFLICT, "Regulator swapped for the 5.0 V variant of the AMS1117 while its output is still the 3V3 rail",
         lambda r: (edit(st(r, "hardware/product.toml"), 'symbol    = "Regulator_Linear:AMS1117-3.3"', 'symbol    = "Regulator_Linear:AMS1117-5.0"'),
                    edit(st(r, "hardware/product.toml"), 'mpn       = "AMS1117-3.3"', 'mpn       = "AMS1117-5.0"'),
                    edit(st(r, "hardware/product.toml"), 'lcsc      = "C6186"', 'lcsc      = "C6187"')), True),
    Case("I12", S, "circuit", CONFLICT, "LED data input wired straight to the 3.3 V GPIO net, skipping the level-shifting buffer",
         lambda r: edit(st(r, "hardware/product.toml"),
                        'pins      = { VDD = "VBUS", VSS = "GND", DIN = "LED_DIN_5V" }',
                        'pins      = { VDD = "VBUS", VSS = "GND", DIN = "LED_DIN" }'), True),
    Case("I13", S, "hardware", INCOMPLETE, "Decoupling caps placed near mcu.VDD, a pin the RP2040 does not have",
         lambda r: edit(st(r, "hardware/product.toml"), 'near      = "mcu.IOVDD"', 'near      = "mcu.VDD"'), True),
    Case("I14", S, "mechanical", CONFLICT, "Outline drawing shortened to 30x40, too short for the 79 mm board",
         lambda r: (edit(st(r, "hardware/outline.svg"), 'viewBox="0 0 30 70"', 'viewBox="0 0 30 40"'),
                    edit(st(r, "hardware/outline.svg"), "V65 L25,70 H5 L0,65 V1", "V35 L25,40 H5 L0,35 V1")), True),
    Case("I15", S, "mechanical", INCOMPLETE, "through_wall removed from the USB-C socket, so the case has no plug window",
         lambda r: edit_re(st(r, "hardware/product.toml"), r"^through_wall = true.*\n", ""), True),
    Case("I16", S, "protocol", CONFLICT, "New status message declared with kind 1, the reading's kind",
         lambda r: edit(st(r, "protocol.toml"), "[[band]]",
                        '[[message]]\n'
                        'name   = "status"\n'
                        'kind   = 1\n'
                        'doc    = "Uptime, sent when the page connects."\n'
                        'fields = [\n'
                        '  { name = "uptime_s", type = "u32", doc = "Seconds since power-on" },\n'
                        ']\n\n[[band]]'), True),
    Case("I17", S, "protocol", STALE, "USB product ID changed in protocol.toml, not re-derived",
         lambda r: edit(st(r, "protocol.toml"), "product_id = 0x000a", "product_id = 0x0010"), False),
    Case("I18", S, "firmware", BYPASS, "Firmware enumerates with literal USB IDs instead of the generated constants",
         lambda r: edit(st(r, "firmware/rp2040/src/main.rs"),
                        "Config::new(USB_VENDOR_ID, USB_PRODUCT_ID)", "Config::new(0x2e8a, 0x000a)"), True),
    Case("I19", S, "hardware", HAND, "Generated BOM edited by hand to order a sixth decoupling cap",
         lambda r: edit(st(r, "hardware/generated/bom.csv"),
                        "C14663,Capacitor_SMD:C_0603_1608Metric,5,0.01771",
                        "C14663,Capacitor_SMD:C_0603_1608Metric,6,0.01771"), False),
    Case("I20", S, "web", BYPASS, "Page scales the reading by a hand-typed /100 instead of the declared scale",
         lambda r: (edit(st(r, "web/src/reading.ts"),
                         "import { READING_UNITS, decodeReading, readingBand, readingValues } from",
                         "import { READING_UNITS, decodeReading, readingBand } from"),
                    edit(st(r, "web/src/reading.ts"),
                         "  const v = readingValues(r)\n  return { celsius: v.centiCelsius, percentRh: v.centiPercentRh }",
                         "  return { celsius: r.centiCelsius / 100, percentRh: r.centiPercentRh / 100 }")), True),
]
