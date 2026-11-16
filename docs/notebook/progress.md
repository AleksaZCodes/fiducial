# Progress

*What each release moved, against the open Fiducial tasks. Newest first.*

## 0.30.0 · 2026-11-16

- Keystatic's GitHub storage is declared and type-checked against the real
  package. The editor's deploy and its GitHub App are not built.

## 0.29.0 · 2026-11-16

- Pages CMS is the third editor: `.pages.yml`, derived from the content model and
  held to Pages CMS's own schema in CI. Hosted editing through Keystatic's GitHub
  mode and Tina Cloud is still open.

## 0.28.0 · 2026-11-16

Phase 8 closes: the topology declares CAN, CAN-FD and RS-485 bit rates and a
Linux node's real-time settings, derives them into constants, and `fiducial-rt`
applies them on the kernel. Latency on `PREEMPT_RT` and the controller drivers
remain open.

## 0.27.0 · 2026-11-16

- CAN-FD frames, ISO-TP segmentation and Modbus RTU with RS-485 direction control,
  in `fiducial-protocol`. Proved over an in-memory bus and a virtual serial port;
  the kernel here has no CAN and there is no transceiver.

## 0.26.0 · 2026-11-16

- The response headers are one table, derived into every framework's file and the
  Caddyfile; HSTS preload is a declared choice. SvelteKit's hand-written headers are
  the one copy left.

## 0.25.0 · 2026-11-16

- The React Router starter is full stack: loader, action, form, server-only module
  and a resource route, proved on the built app. Remix is React Router v8 now; Remix 3
  is a different, non-React project and is not added.

## 0.24.0 · 2026-11-15

- `fid add app start`: TanStack Start on Workers, with its headers set by a custom
  server entry. Every framework in the "stack chosen per app" list now has a template.

## 0.23.0 · 2026-11-15

- `fid add app router`: React Router 8 on Workers through the Cloudflare Vite plugin,
  with its security headers set by the Worker and checked by `fid doctor`.

## 0.22.0 · 2026-11-15

- `fid add app api`: a Hono API for a Worker and for Node, with a typed client.
- `orm-drizzle`: the TypeScript schema is the declaration, and its SQL feeds the
  existing migration runner. Capabilities can target another app's `package.json`.

## 0.21.0 · 2026-11-15

- `[apps.<name>]`: each app declares framework, editor, shell, api and orm, and
  `fid doctor` checks the choice can be built and fits the others.
- `fid add app astro`: a static Astro site, deployed as static assets. Verified on
  a fresh product through install, typecheck, lint, build and a wrangler dry run.

## 0.20.0 · 2026-11-15

- The web app as a static export served by Caddy (`[deploy] static_dir`), built
  from the Next.js template with `pnpm build:static`. Phase 6 is done, so the
  distributed, self-hosted group is complete except CAN, which waits for a product.

## 0.19.0 · 2026-11-15

- The self-hosted server stack is complete: `sqlite`, `native` auth and `ollama`
  are implemented in Rust (`fiducial-adapters` features) and selectable, with no
  TypeScript class. Phase 5 of distributed products is done.

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
