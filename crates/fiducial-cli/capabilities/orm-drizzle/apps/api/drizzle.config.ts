import { defineConfig } from "drizzle-kit";

// The schema is `src/db/schema.ts`: that file is the declaration, and nothing
// else describes these tables. `drizzle-kit generate` turns a change to it into
// SQL in the platform's `migrations/` directory, where `fid derive` embeds it in
// `src/migrations.generated.ts` for the `Migrator` to apply. There is no second,
// hand-written copy of the tables.
//
// SQLite is D1's dialect and the self-hosted target's.
export default defineConfig({
  dialect: "sqlite",
  schema: "./src/db/schema.ts",
  out: "../../migrations",
});
