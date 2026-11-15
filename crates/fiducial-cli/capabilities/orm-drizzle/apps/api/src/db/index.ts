import { drizzle } from "drizzle-orm/d1";
import * as schema from "./schema";

/**
 * Typed queries over the `database` contract's D1 binding (`env.DB`, the same
 * binding the platform's `D1Database` adapter reads).
 *
 *   const db = createDb(c.env.DB);
 *   const rows = await db.select().from(schema.notes);
 *
 * This is a layer over the contract, not a replacement for it: the `Migrator`
 * and the generated factory stay, and a route that wants a raw statement uses
 * them.
 */
export function createDb(d1: D1Database) {
  return drizzle(d1, { schema });
}

export type Db = ReturnType<typeof createDb>;
export { schema };
