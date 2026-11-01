# Fourth blind set: 20 realistic mistakes.
#
# Written on 2026-11-01 by a separate agent that read only the two products'
# files: a web product scaffolded fresh with
#   fid new --locales en,sr w
#   fid add capability {i18n,content,brand,seo,legal,design,adapters,deploy,migrations,identity}
#   fid derive
# and examples/sensor-stick/ (plus the sensor-stick job of the repo's root
# .github/workflows/ci.yml, which the brief named as the stick's gate).
# Aimed at five classes, three cases each, plus five free cases:
#   1 shadow copy        F1  F2  F3
#   2 gate tampering     F4  F5  F6
#   3 weak types         F7  F8  F9
#   4 outside-world rule F10 F11 F12
#   5 untracked inputs   F13 F14 F15
#   6 free               F16 F17 F18 F19 F20
# Committed before its first run. `fid derive --check` was never run against
# any of these cases; the only check made was that every edit applies.
#
# Breaches of the blindness rules (minor, listed for completeness):
#   - The repository's CLAUDE.md was injected into the agent's context
#     automatically; it describes the plugin, the guard hook and the
#     /fiducial:* commands. No platform source, spec or AGENTS.md at the repo
#     root was read.
#   - Listing docs/paper/markers/ to confirm the output directory showed the
#     file names independent_cases.py and results-baseline.{json,md}. Their
#     contents were not opened.
#   - Reading the stick's job in the root ci.yml also showed the neighbouring
#     school-building job's steps (fid derive --check, a re-derive diff check).
#   - The products' own files carry platform commentary (pipeline comments,
#     generated-file headers, design-system.md's note that derive measures
#     contrast pairs, a `fid: allow-band` marker in web/src/reading.test.ts).
#     Those were read as product files, as allowed.

FOURTH = [
    # ── 1 · Shadow copy ─────────────────────────────────────────────────────
    Case(
        "F1", W, "brand", BYPASS,
        "A hand-written site footer types the legal name, contact email, domain and primary colour as literals instead of importing src/generated/brand.ts.",
        lambda r: (
            (r / "apps/web/src/components/site-footer.tsx").write_text(
                'export function SiteFooter() {\n'
                '  return (\n'
                '    <footer className="type-small border-t py-8" style={{ borderColor: "#0EA5E9" }}>\n'
                '      <p>© {new Date().getFullYear()} Example LLC. All rights reserved.</p>\n'
                '      <p>\n'
                '        Questions? <a href="mailto:hello@example.com">hello@example.com</a> ·{" "}\n'
                '        <a href="https://example.com/privacy">Privacy</a>\n'
                '      </p>\n'
                '    </footer>\n'
                '  );\n'
                '}\n'
            ),
        ),
        False,
    ),
    Case(
        "F2", S, "firmware", BYPASS,
        "The firmware's LED colour matches on hand-typed humidity counts (3500..=6000) instead of the generated comfortable() band from protocol.toml.",
        lambda r: edit(
            st(r, "firmware/rp2040/src/main.rs"),
            "    match comfortable(r.centi_percent_rh) {\n"
            "        Band::Inside => RGB8::new(0, 24, 0),\n"
            "        Band::Below => RGB8::new(0, 0, 24),\n"
            "        Band::Above => RGB8::new(24, 0, 0),\n"
            "    }\n",
            "    // 35–60 %RH, in hundredths.\n"
            "    match r.centi_percent_rh {\n"
            "        3500..=6000 => RGB8::new(0, 24, 0),\n"
            "        0..=3499 => RGB8::new(0, 0, 24),\n"
            "        _ => RGB8::new(24, 0, 0),\n"
            "    }\n",
        ),
        False,
    ),
    Case(
        "F3", S, "web", BYPASS,
        "The page narrows its Web Serial filter with the USB product ID typed as a literal 0x000a, beside the imported vendor ID, so replacing the placeholder IDs in protocol.toml leaves the page unable to find the stick.",
        lambda r: edit(
            st(r, "web/src/main.ts"),
            "filters: [{ usbVendorId: USB_VENDOR_ID }]",
            "filters: [{ usbVendorId: USB_VENDOR_ID, usbProductId: 0x000a }]",
        ),
        False,
    ),

    # ── 2 · Gate tampering ──────────────────────────────────────────────────
    Case(
        "F4", W, "ci", BYPASS,
        "The derive-check step in the web product's CI is marked continue-on-error, so a stale artifact shows a warning and the build stays green.",
        lambda r: edit(
            r / ".github/workflows/ci.yml",
            "      - name: fid derive --check\n        run: fid derive --check\n",
            "      - name: fid derive --check\n        run: fid derive --check\n"
            "        continue-on-error: true  # flaky on fresh runners, do not block merges\n",
        ),
        False,
    ),
    Case(
        "F5", W, "ci", BYPASS,
        "The CI step is changed from `fid derive --check` to plain `fid derive`, which regenerates everything on the runner and can never fail on staleness.",
        lambda r: edit(
            r / ".github/workflows/ci.yml",
            "      - name: fid derive --check\n        run: fid derive --check\n",
            "      - name: fid derive\n        run: fid derive  # regenerate rather than fail on a forgotten derive\n",
        ),
        False,
    ),
    Case(
        "F6", S, "ci", BYPASS,
        "In the root CI's sensor-stick job the derive-check step is moved after hardware/build.sh, which runs `fid derive --pipeline hardware` itself, so stale hardware artifacts are regenerated before they are checked.",
        lambda r: (
            edit(
                r / ".github/workflows/ci.yml",
                "      - name: fid derive --check (every derived artifact matches the declaration)\n"
                "        run: fid derive --check\n\n",
                "",
            ),
            edit(
                r / ".github/workflows/ci.yml",
                "        run: SKIP_RENDER=1 hardware/build.sh\n",
                "        run: SKIP_RENDER=1 hardware/build.sh\n\n"
                "      # After the build, so everything it generates is in place first.\n"
                "      - name: fid derive --check (every derived artifact matches the declaration)\n"
                "        run: fid derive --check\n",
            ),
        ),
        False,
    ),

    # ── 3 · Weak types ──────────────────────────────────────────────────────
    Case(
        "F7", S, "hardware", MEANING,
        "The AHT20's I²C address is changed to 0x70, the 8-bit write address in the datasheet's table, where the field holds the 7-bit address 0x38.",
        lambda r: edit(
            st(r, "hardware/product.toml"),
            "i2c_address = 0x38       # AHT20 datasheet: fixed 7-bit address",
            "i2c_address = 0x70       # AHT20 datasheet: address byte 0x70 (write) / 0x71 (read)",
        ),
        True,
    ),
    Case(
        "F8", S, "protocol", RULE,
        "The humidity field's `scale` key is misspelled `sacle`, so the reading and the comfortable band lose their ×0.01 scale.",
        lambda r: edit(
            st(r, "protocol.toml"),
            'doc = "Relative humidity, %RH × 100", scale = 0.01, unit = "%RH" }',
            'doc = "Relative humidity, %RH × 100", sacle = 0.01, unit = "%RH" }',
        ),
        True,
    ),
    Case(
        "F9", W, "brand", MEANING,
        "[brand] domain is given with its scheme (\"https://example.com\"), where every derivation prefixes https:// itself.",
        lambda r: edit(
            r / "fiducial.toml",
            'domain = "example.com"',
            'domain = "https://example.com"',
        ),
        True,
    ),

    # ── 4 · Outside-world rules ─────────────────────────────────────────────
    Case(
        "F10", S, "hardware", RULE,
        "The I²C pull-ups are changed from 4.7 kΩ to 470 Ω for faster edges: 3.3 V / 470 Ω ≈ 7 mA, above the 3 mA sink current the I²C standard allows (LCSC part left unchanged).",
        lambda r: (
            edit(
                st(r, "hardware/product.toml"),
                'value     = "4.7k"\nlcsc      = "C23162"\nnear      = "mcu.GPIO4"',
                'value     = "470R"\nlcsc      = "C23162"\nnear      = "mcu.GPIO4"',
            ),
            edit(
                st(r, "hardware/product.toml"),
                'value     = "4.7k"\nlcsc      = "C23162"\nnear      = "mcu.GPIO5"',
                'value     = "470R"\nlcsc      = "C23162"\nnear      = "mcu.GPIO5"',
            ),
        ),
        True,
    ),
    Case(
        "F11", S, "hardware", RULE,
        "Track width and clearance are cut to 0.075 mm (3 mil) to make the QFN escape easier, below what the declared fab's standard two-layer process can make.",
        lambda r: (
            edit(
                st(r, "hardware/product.toml"),
                "track_mm     = 0.15                 # the RP2040's 0.4 mm-pitch QFN escapes at 0.15 / 0.127",
                "track_mm     = 0.075                # thinner so the QFN escapes without vias",
            ),
            edit(
                st(r, "hardware/product.toml"),
                "clearance_mm = 0.127                # JLCPCB's standard two-layer process",
                "clearance_mm = 0.075                # JLCPCB's standard two-layer process",
            ),
        ),
        True,
    ),
    Case(
        "F12", W, "legal", RULE,
        "Analytics and marketing cookie categories are declared, and the banner initialises its checkboxes with every optional category selected: pre-ticked consent, invalid under GDPR/ePrivacy.",
        lambda r: (
            edit(
                r / "fiducial.toml",
                'cookie_categories = ["necessary"]',
                'cookie_categories = ["necessary", "analytics", "marketing"]',
            ),
            edit(
                r / "apps/web/src/components/cookie-consent.tsx",
                "  const [selected, setSelected] = useState<CookieCategory[]>([]);",
                "  const [selected, setSelected] = useState<CookieCategory[]>([...categories]);",
            ),
        ),
        True,
    ),

    # ── 5 · Untracked inputs ────────────────────────────────────────────────
    Case(
        "F13", W, "seo", INCOMPLETE,
        "scripts/derive-seo.mjs is edited to merge extra pages from a new apps/web/src/app/extra-pages.json into the sitemap; the file is not among the seo pipeline's declared inputs.",
        lambda r: (
            edit(
                r / "scripts/derive-seo.mjs",
                "const seo = cfg.seo ?? {};\n",
                "const seo = cfg.seo ?? {};\n"
                "// Marketing pages the content team maintains, merged in so they reach the sitemap.\n"
                'const EXTRA = "apps/web/src/app/extra-pages.json";\n'
                "if (existsSync(at(EXTRA))) seo.pages = [...(seo.pages ?? [\"/\"]), ...JSON.parse(read(EXTRA))];\n",
            ),
            (r / "apps/web/src/app/extra-pages.json").write_text(
                '[\n  { "path": "/pricing", "title": "meta.title", "description": "meta.description" }\n]\n'
            ),
        ),
        True,
    ),
    Case(
        "F14", W, "content", INCOMPLETE,
        "scripts/derive-content.mjs is edited to give every entry an author from [brand] trading_name, but the content pipeline declares only fiducial.toml#i18n, so a later brand rename leaves content.ts stale without the gate noticing.",
        lambda r: edit(
            r / "scripts/derive-content.mjs",
            '      return { slug: f.replace(/\\.md$/, ""), ...meta, ...(wantsBody ? { body } : {}) };',
            "      // Byline: every entry is by the company unless its frontmatter says otherwise.\n"
            '      return { slug: f.replace(/\\.md$/, ""), author: cfg.brand?.trading_name ?? "", ...meta, ...(wantsBody ? { body } : {}) };',
        ),
        True,
    ),
    Case(
        "F15", S, "hardware", INCOMPLETE,
        "hardware/cad.py is edited to overlay a new hardware/case-tweaks.json on the solved layout (1.0 mm walls, 0.1 mm clearance), an input nothing declares that overrides product.toml's [case].",
        lambda r: (
            edit(
                st(r, "hardware/cad.py"),
                "    L = json.loads(LAYOUT.read_text())\n",
                "    L = json.loads(LAYOUT.read_text())\n"
                "    # Printer-specific tweaks (our printer runs walls fat), applied over the solved layout.\n"
                '    tweaks = ROOT / "hardware" / "case-tweaks.json"\n'
                "    if tweaks.exists():\n"
                '        L["case"].update(json.loads(tweaks.read_text()))\n',
            ),
            st(r, "hardware/case-tweaks.json").write_text('{"wall_mm": 1.0, "clearance_mm": 0.1}\n'),
        ),
        False,
    ),

    # ── 6 · Free ────────────────────────────────────────────────────────────
    Case(
        "F16", S, "firmware", CONFLICT,
        "memory.x is changed to a 16 MB flash region (copied from a W25Q128 board) while the declared flash part is a 32 Mbit (4 MB) W25Q32, so a large image links and then cannot fit on the chip.",
        lambda r: edit(
            st(r, "firmware/rp2040/memory.x"),
            "    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100",
            "    FLASH : ORIGIN = 0x10000100, LENGTH = 16384K - 0x100",
        ),
        False,
    ),
    Case(
        "F17", W, "design", MEANING,
        "The muted-foreground grey is lightened, and the two contrast pairs that then fail are relabelled `large-text` with a 3.0 minimum, although that colour is used for captions and small secondary text.",
        lambda r: (
            edit(
                r / "design-system.md",
                'muted-foreground     = "oklch(0.47 0 0)"        # secondary text, captions',
                'muted-foreground     = "oklch(0.62 0 0)"        # secondary text, captions',
            ),
            edit(
                r / "design-system.md",
                '{ fg = "muted-foreground",   bg = "background", min = 4.5, note = "secondary text; the pair that fails most often" },',
                '{ fg = "muted-foreground",   bg = "background", min = 3.0, kind = "large-text", note = "secondary text; the pair that fails most often" },',
            ),
            edit(
                r / "design-system.md",
                '{ fg = "muted-foreground",   bg = "muted",      min = 4.5, note = "the same text on a section band" },',
                '{ fg = "muted-foreground",   bg = "muted",      min = 3.0, kind = "large-text", note = "the same text on a section band" },',
            ),
        ),
        True,
    ),
    Case(
        "F18", W, "i18n", CONFLICT,
        "A hand-written Next.js middleware keeps its own locale list and redirects to a default of \"en\", while [i18n] declares the default as \"sr\".",
        lambda r: (r / "apps/web/src/middleware.ts").write_text(
            'import { NextResponse, type NextRequest } from "next/server";\n'
            "\n"
            'const LOCALES = ["en", "sr"] as const;\n'
            'const DEFAULT_LOCALE = "en";\n'
            "\n"
            "export function middleware(req: NextRequest) {\n"
            "  const { pathname } = req.nextUrl;\n"
            '  const first = pathname.split("/")[1];\n'
            "  if ((LOCALES as readonly string[]).includes(first)) return NextResponse.next();\n"
            "  const url = req.nextUrl.clone();\n"
            "  url.pathname = `/${DEFAULT_LOCALE}${pathname}`;\n"
            "  return NextResponse.redirect(url);\n"
            "}\n"
            "\n"
            'export const config = { matcher: ["/((?!_next|api|.*\\\\..*).*)"] };\n'
        ),
        False,
    ),
    Case(
        "F19", S, "hardware", RULE,
        "The LED level-shifting buffer's supply pin is moved from VBUS to 3V3, so its output tops out at 3.3 V, below the WS2812B's 3.5 V input-high threshold at 5 V; its decoupling cap stays on VBUS.",
        lambda r: edit(
            st(r, "hardware/product.toml"),
            'pins      = { 1 = "GND", 2 = "LED_DIN", 3 = "GND", 4 = "LED_DIN_5V", 5 = "VBUS" }',
            'pins      = { 1 = "GND", 2 = "LED_DIN", 3 = "GND", 4 = "LED_DIN_5V", 5 = "3V3" }',
        ),
        True,
    ),
    Case(
        "F20", W, "migrations", CONFLICT,
        "A hand-written migrations/0002_profiles.sql is added, then [identity] audit = true is turned on, which derives migrations/0002_audit.sql: two migrations share number 0002.",
        lambda r: (
            (r / "migrations/0002_profiles.sql").write_text(
                "CREATE TABLE IF NOT EXISTS profiles (\n"
                "  user_id      TEXT PRIMARY KEY,\n"
                "  display_name TEXT NOT NULL,\n"
                "  created_at   TEXT NOT NULL\n"
                ");\n"
            ),
            edit(
                r / "fiducial.toml",
                '[identity]\ntable = "grants"\n',
                '[identity]\ntable = "grants"\naudit = true\n',
            ),
        ),
        True,
    ),
]
