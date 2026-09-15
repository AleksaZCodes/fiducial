---
name: migrations
description: Schema migrations — ordered, idempotent, and checked for drift
---

# Schema migrations

SQL applied to a product's database. **Not** the codemod migrations `fid
upgrade` applies to source files — those are `migration.rs`, these are
`schema.rs`, and they share nothing but a word.

## Adding one

1. Write `migrations/NNNN_slug.sql`, where `NNNN` is the next number.
2. Run `fid derive`. It regenerates `src/migrations.generated.ts`.
3. Apply with the `Migrator` from `@fiducial/adapters/migrate`.

```ts
import { Migrator } from "@fiducial/adapters/migrate";
import { migrations, ledgerTable } from "./migrations.generated.js";

const report = await new Migrator(db, migrations, { ledgerTable, dialect }).apply();
```

## Rules the generator enforces

| Rule | Why it is not a preference |
|---|---|
| `NNNN_slug.sql`, nothing else in `migrations/` | a `.sql` file that is not a migration is one that silently never runs |
| No two migrations share a number | order is what makes a set replayable, and two files with one number have no order |
| Never edit an applied migration | environments that ran it keep the old schema; new ones get the new one; neither can tell |

The last is not enforceable at generation time — it depends on a live
database — so the runner enforces it. `Migrator.apply()` refuses to run at all
while any applied migration's hash disagrees with the file, because applying
more on top of a schema nobody has reconciled compounds it.

To change an applied migration, **add a new one.**

## Key constraints

- The manifest is generated. Never hand-edit `src/migrations.generated.ts`.
- `dialect` must match `[adapters] database`: Postgres queried with SQLite's
  `?` placeholders is a syntax error on every statement, and it is silent
  until runtime.
- Drift is reported, never repaired. There is no safe automatic answer, and
  the only useful thing the runner can do is make sure a person knows.
