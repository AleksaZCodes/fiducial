// The tables of this product, as TypeScript. This file IS the declaration.
//
// Add a table, then run `pnpm db:generate` in this app: Drizzle writes the SQL
// to `migrations/` and `fid derive` embeds it. Never edit a generated
// migration that has been applied; change this file and generate another.
//
//   import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";
//
//   export const notes = sqliteTable("notes", {
//     id: integer("id").primaryKey({ autoIncrement: true }),
//     body: text("body").notNull(),
//     createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
//   });
//
// It starts empty on purpose: a placeholder table would be a migration that runs.
export {};
