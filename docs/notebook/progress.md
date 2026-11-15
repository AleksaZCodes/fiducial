# Progress

*What each release moved, against the open Fiducial tasks. Newest first.*

## 0.18.0 · 2026-11-15

- `deploy = "compose"`: the self-hosted target is derived: compose services,
  Caddy, and the Mosquitto broker with a per-device topic ACL, from the topology.
  The `sqlite`, `native` auth and `ollama` implementations are still to build.

- `fid add firmware esp32c6-idf`: ESP32-C6 on ESP-IDF (std Rust) beside the Embassy
  one. Its full build is not yet verified: the SDK installer needs a host the
  authoring sandbox blocks.

## 0.17.0 · 2026-11-15

- `fid add firmware esp32c6`: the ESP32-C6 template (esp-hal, `esp-rtos`, Embassy).
- The RP2040 and STM32 templates did not compile in a fresh product (a re-export
  of two names that do not exist, a linker path, a missing critical section).
  Fixed, and `scripts/verify-firmware.sh` now builds every target in CI.

## 0.16.0 · 2026-11-15

- Derived documentation: `topology.md` (nodes by system, buses, topics as a
  sequence) from `fid-topology`, `rules.md` (rules and storage) from
  `fid-monitoring`. Distributed products phase 7 is done.
- The paper was submitted; the roadmap, `SHIPPED.md` and the paper notes say so.

## 0.15.0 · 2026-11-15 ([#123](https://github.com/AleksaZCodes/fiducial/pull/123))

- `fid new <name> --shape site`: a localized Next.js site in one command
  (pages, editor, fonts, the app as its own Worker), verified through an
  OpenNext build and `wrangler deploy --dry-run`.
- Keystatic (default) and Tina replace Decap, derived from one content model.
- Fresh Next.js and SvelteKit products install, typecheck, lint, build and
  deploy; `scripts/verify-web-next.sh`, `scripts/verify-web-svelte.sh`.
- Downstream: Outreach and aleksa.works upgraded; Outreach live.

Moves these open tasks (none closed; each needs its own evidence):

| Task | What moved |
|---|---|
| Build and validate one portable minimal web template | The site shape is that template inside the monorepo; still to do: copy it outside and build with two agents |
| Audit Fiducial against the new vision | A build-it-for-real review found and fixed a dozen gaps; the gap table itself is not written |
| Clarify source, generated and escape-hatch ownership | `supersedes`, `media_field = "text"`, `[site] fonts = "none"` are scoped escape hatches |
| Trace how i18n connects to a running app | The site shape is the trace: catalog → `generated/messages.ts` → `t()` → `[locale]` routes |

## 0.12.0–0.14.0 · 2026-11-11 to 13

Prebuilt `fid` and lean CI; Durable Objects in `[deploy]`; aleksa-works' CMS
work generalized; the language picker above sticky chrome.

## Next

- `fid notebook`: sync this folder with the planning tool it mirrors (the
  Notion API first), so the copy cannot drift. Until then, by hand.
