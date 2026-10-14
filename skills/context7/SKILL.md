---
name: context7
description: Fetch current documentation before writing code against any versioned library, framework, CLI or config format — Next.js, React, Base UI, shadcn, Tailwind, pnpm, Embassy, Tauri, wrangler, anything with a version. Use it before the first line, not after the first error.
---

# Current docs before code

Training data on a library is a snapshot, and the libraries a Fiducial product
uses move fast. What it costs when an agent writes from memory instead, from
this platform's own history:

- **Embassy, Phase 2.** `arch-cortex-m` had become `platform-cortex-m`,
  `integrated-timers` was gone, and `embassy-rp` had jumped from 0.3 to 0.10. A
  full trial-and-error loop.
- **Base UI, 0.9.4.** A template depended on `@base-ui-components/react ^1.0.0`
  — a version that was never published; the stable package is `@base-ui/react`.
  Every fresh product's `pnpm install` failed.
- **pnpm 11, 0.9.5.** `onlyBuiltDependencies` is silently ignored; the setting
  is `allowBuilds`. A product could not run a single script.

## How

The context7 MCP server:

1. `resolve-library-id` with the library's name and what you are about to do.
   Prefer the result whose versions include the one the product pins — read
   `package.json`, `Cargo.toml`, or the lockfile first.
2. `query-docs` with that id (with `/<version>` when there is one) and one
   specific question. One concept per query.

Ask before writing, for: an API you have not used in this product yet, a
configuration file's keys, a CLI's flags, a migration between major versions,
and any error message that names the library.

## When context7 is not there

It can fail to connect at session start. **Say so** — to the person, in one
line — and then go to the primary source from the shell, never to memory:

- the registry: `https://registry.npmjs.org/<pkg>/latest`, `https://crates.io/api/v1/crates/<crate>`
- the published schema, where there is one (`https://ui.shadcn.com/schema.json`)
- the installed package's own types: `node_modules/<pkg>/**/*.d.ts`, `cargo doc`
- the library's source at the pinned tag: `gh api repos/<owner>/<repo>/contents/<path>?ref=<tag>`

A version you remember is a version to check, not a version to write.
