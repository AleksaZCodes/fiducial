# Skill: orm-drizzle

**Capability:** `orm-drizzle` · **Platform:** Fiducial {{version}}

---

## What this capability adds

Typed queries for the API app (`apps/api`) over the platform's `database` contract.
The point is that **the TypeScript schema is the declaration** and nothing else
describes the tables:

| File | Is |
|---|---|
| `apps/api/src/db/schema.ts` | the tables, in TypeScript. **The declaration** |
| `apps/api/drizzle.config.ts` | points Drizzle at that schema and at `migrations/` |
| `apps/api/src/db/index.ts` | `createDb(env.DB)`: typed queries over the D1 binding |
| `migrations/NNNN_slug.sql` | what `pnpm db:generate` writes from a changed schema |
| `src/migrations.generated.ts` | what `fid derive` builds from those files, for the `Migrator` |

Drizzle's SQL feeds the existing migration runner, so there is no second,
hand-written copy of the tables, and the runner's ordering, idempotency and drift
checks apply to what Drizzle generated.

## Declare it

```toml
[adapters]
database = "d1"          # or "sqlite" for the self-hosted target

[apps.api]
framework = "hono"
orm       = "drizzle"
```

`fid doctor` refuses `orm = "drizzle"` while `[adapters] database` is `none`: an
ORM is a layer over a database.

## Change the schema

```sh
# edit apps/api/src/db/schema.ts, then, in apps/api:
pnpm db:generate         # writes migrations/NNNN_slug.sql, only if the schema changed
cd ../.. && fid derive   # rebuilds src/migrations.generated.ts
```

A migration that has been applied is never edited: change the schema and
generate another. `fid derive --check` fails when the manifest is stale.

## What this does not do

- **No Postgres yet.** `drizzle.config.ts` is SQLite, which is D1's dialect and
  the self-hosted target's. Supabase would need `dialect = "postgresql"` and the
  `pg` driver, and a second migration flavour in the runner.
- **No `sqlite` driver for the Node entry.** `createDb` takes a D1 binding. The
  self-hosted server reads SQLite through the Rust adapter; a Node-side driver
  (`better-sqlite3`) is a choice to make when a Node-served app needs one.
- **No seed data and no studio.** Drizzle Studio is a separate tool and is not
  wired.

## Guard rules activated

| Rule | What it prevents |
|---|---|
| `no-unpinned-cli-fetch` | Fetching tools without version pinning |
