/**
 * Database contract — relational storage: queries, migrations, transactions.
 *
 * Mirrors `fiducial_adapters::database` in Rust. Any method not shared by
 * D1, Supabase, Neon and Postgres stays on the vendor type, not here.
 *
 * The `None*` implementation mirrors the Rust side exactly. The real vendor
 * (`D1Database`, below) does not: D1 is reached through a Workers runtime
 * binding, which exists only in the TypeScript/Workers half of a product.
 * There is no Tauri-reachable equivalent yet — see the module doc on
 * `D1Database` for what that would take.
 */

import postgres, { type Sql } from "postgres";

/** A SQL parameter value — the intersection across all target vendors. */
export type SqlValue = null | boolean | number | string | Uint8Array;

/** One row from a query result. */
export interface Row {
  /** Return the value for a column, or undefined if absent. */
  get(column: string): SqlValue | undefined;
  /** All column names in this row. */
  columns(): string[];
}

export class MapRow implements Row {
  constructor(private readonly data: Record<string, SqlValue>) {}
  get(column: string): SqlValue | undefined {
    return this.data[column];
  }
  columns(): string[] {
    return Object.keys(this.data);
  }
}

/** Result of a write statement. */
export interface WriteResult {
  rowsAffected: number;
}

/** Errors a `Database` implementation may return. */
export class DatabaseError extends Error {
  constructor(
    message: string,
    public readonly code?: string,
  ) {
    super(message);
    this.name = "DatabaseError";
  }
}

/**
 * Relational database contract.
 *
 * Designed to be satisfied by D1, Supabase, Neon, and Postgres adapters.
 * Each returns a Promise — callers `await` them; implementations handle the
 * network or SDK call internally.
 */
export interface Database {
  /** Execute a write statement (INSERT, UPDATE, DELETE, DDL). */
  execute(sql: string, params: SqlValue[]): Promise<WriteResult>;

  /** Run a SELECT query and return all matching rows. */
  query(sql: string, params: SqlValue[]): Promise<Row[]>;

  /** Run a SELECT and return the first row, or null if empty. */
  queryOne(sql: string, params: SqlValue[]): Promise<Row | null>;

  /**
   * Run a sequence of statements as an atomic batch.
   *
   * Returns one WriteResult per statement. Atomicity semantics match each
   * vendor — D1 uses its batch API; Supabase and Postgres use a transaction.
   */
  batch(
    statements: Array<{ sql: string; params: SqlValue[] }>,
  ): Promise<WriteResult[]>;
}

// ── None implementation ───────────────────────────────────────────────────────

/**
 * No-op database — all writes succeed silently; all queries return empty.
 *
 * `database = "none"` is a real, working selection. A product that declares
 * it compiles, runs, and can be tested without a live database.
 */
export class NoneDatabase implements Database {
  // Accepts and ignores `env` so every vendor class in this contract shares
  // one constructor shape — `new {Class}(env)` — for the generated factory
  // in `src/adapters.generated.ts` to call uniformly.
  constructor(_env?: unknown) {}

  async execute(_sql: string, _params: SqlValue[]): Promise<WriteResult> {
    return { rowsAffected: 0 };
  }

  async query(_sql: string, _params: SqlValue[]): Promise<Row[]> {
    return [];
  }

  async queryOne(_sql: string, _params: SqlValue[]): Promise<Row | null> {
    return null;
  }

  async batch(
    statements: Array<{ sql: string; params: SqlValue[] }>,
  ): Promise<WriteResult[]> {
    return statements.map(() => ({ rowsAffected: 0 }));
  }
}

// ── D1 ─────────────────────────────────────────────────────────────────────

/**
 * D1 — Cloudflare's Workers-native SQLite database, reached through a
 * binding (`env.DB`), never over the network from outside a Worker.
 *
 * `D1Binding` mirrors the shape of `@cloudflare/workers-types`'
 * `D1Database` (prepare/bind/run/all/first/batch) without depending on that
 * package at the type level, so this file compiles standalone. A product
 * that installs `@cloudflare/workers-types` gets full binding types for
 * `env`; this adapter only needs the methods it actually calls.
 *
 * **Binding convention:** the adapter reads `env.DB`. `fid derive` cannot
 * know a product's chosen binding name — `wrangler.toml` is hand-edited —
 * so `DB` is the one name every scaffolded `worker-cloudflare` product is
 * expected to use, matching Cloudflare's own quickstart convention. A
 * product that binds D1 under a different name passes its own `env` shape
 * and adjusts, or wraps this class.
 *
 * **Reachable only from a Worker.** D1 has no direct-dial client for a
 * Tauri desktop backend — the binding exists solely inside the Workers
 * runtime. A Rust-side D1 adapter would have to go through Cloudflare's D1
 * HTTP API (a general Cloudflare API endpoint, authenticated by API token)
 * instead of a binding; nothing in this repository needs that yet, so it is
 * not built. See `docs/specs/2026-09-15-cloudflare-adapter-set.md`.
 */
interface D1Binding {
  prepare(sql: string): D1PreparedStatement;
  batch(statements: D1PreparedStatement[]): Promise<D1Result[]>;
}

interface D1PreparedStatement {
  bind(...values: unknown[]): D1PreparedStatement;
  run(): Promise<D1Result>;
  all(): Promise<D1Result>;
  first(): Promise<Record<string, unknown> | null>;
}

interface D1Result {
  results?: Record<string, unknown>[];
  success: boolean;
  meta: { changes?: number; [key: string]: unknown };
}

export class D1Database implements Database {
  private readonly db: D1Binding;

  constructor(env: { DB?: D1Binding }) {
    if (!env?.DB) {
      throw new DatabaseError(
        "D1Database: no `DB` binding on env — add a [[d1_databases]] block " +
          'with `binding = "DB"` to wrangler.toml',
      );
    }
    this.db = env.DB;
  }

  private stmt(sql: string, params: SqlValue[]): D1PreparedStatement {
    return this.db.prepare(sql).bind(...params);
  }

  async execute(sql: string, params: SqlValue[]): Promise<WriteResult> {
    try {
      const result = await this.stmt(sql, params).run();
      return { rowsAffected: result.meta.changes ?? 0 };
    } catch (err) {
      throw new DatabaseError(`D1 execute failed: ${String(err)}`);
    }
  }

  async query(sql: string, params: SqlValue[]): Promise<Row[]> {
    try {
      const result = await this.stmt(sql, params).all();
      return (result.results ?? []).map(
        (r) => new MapRow(r as Record<string, SqlValue>),
      );
    } catch (err) {
      throw new DatabaseError(`D1 query failed: ${String(err)}`);
    }
  }

  async queryOne(sql: string, params: SqlValue[]): Promise<Row | null> {
    try {
      const row = await this.stmt(sql, params).first();
      return row ? new MapRow(row as Record<string, SqlValue>) : null;
    } catch (err) {
      throw new DatabaseError(`D1 queryOne failed: ${String(err)}`);
    }
  }

  async batch(
    statements: Array<{ sql: string; params: SqlValue[] }>,
  ): Promise<WriteResult[]> {
    try {
      const results = await this.db.batch(
        statements.map(({ sql, params }) => this.stmt(sql, params)),
      );
      return results.map((r) => ({ rowsAffected: r.meta.changes ?? 0 }));
    } catch (err) {
      throw new DatabaseError(`D1 batch failed: ${String(err)}`);
    }
  }
}

// ── Supabase (Postgres) ─────────────────────────────────────────────────────

/**
 * SupabaseDatabase — direct Postgres connection to a Supabase project,
 * using `postgres.js` and a connection string.
 *
 * **Why direct Postgres, not the Supabase JS client's REST API.** The
 * `Database` contract requires raw SQL — `execute`, `query`, `queryOne`,
 * `batch` — and `supabase-js` exposes PostgREST, not SQL. More importantly,
 * `batch` promises atomicity, and PostgREST has no cross-request transaction.
 * A direct Postgres connection satisfies both with `BEGIN`/`COMMIT`.
 *
 * **Credentials:** `SUPABASE_DB_URL` — the full Postgres connection string
 * from Project Settings → Database → Connection string (URI tab) in the
 * Supabase dashboard. It looks like
 * `postgresql://postgres:password@db.PROJECT.supabase.co:5432/postgres`.
 * Set it with `wrangler secret put SUPABASE_DB_URL`. This is distinct from
 * `SUPABASE_ANON_KEY` (which `auth = "supabase"` uses for the REST API):
 * a product using both holds two credentials to the same Supabase project,
 * one for each channel.
 *
 * **`neon` and `postgres` fall out of this adapter for free.** All three
 * vendors speak the Postgres wire protocol; the only difference is the
 * connection string env var (`NEON_DB_URL`, `DATABASE_URL`). When those
 * vendors are promoted from candidates, they will be thin aliases of this
 * class reading different env vars.
 *
 * **Cloudflare Workers:** `postgres.js` supports Workers TCP sockets via
 * Cloudflare's `connect()` API. SSL (`ssl: "require"`) is mandatory for
 * Supabase; Cloudflare Workers enforce TLS on outbound TCP.
 */
export class SupabaseDatabase implements Database {
  private readonly sql: Sql;

  constructor(env: { SUPABASE_DB_URL?: string }) {
    if (!env.SUPABASE_DB_URL) {
      throw new DatabaseError(
        "SupabaseDatabase: SUPABASE_DB_URL is not set — set it to the Postgres " +
          "connection string from Project Settings → Database → Connection string " +
          "in the Supabase dashboard, using `wrangler secret put SUPABASE_DB_URL`",
      );
    }
    this.sql = postgres(env.SUPABASE_DB_URL, { ssl: "require", max: 1 });
  }

  async execute(sql: string, params: SqlValue[]): Promise<WriteResult> {
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const result = await this.sql.unsafe(sql, params as any[]);
      return { rowsAffected: result.count };
    } catch (err) {
      throw new DatabaseError(`SupabaseDatabase execute failed: ${String(err)}`);
    }
  }

  async query(sql: string, params: SqlValue[]): Promise<Row[]> {
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const rows = await this.sql.unsafe(sql, params as any[]);
      return rows.map((r) => new MapRow(r as Record<string, SqlValue>));
    } catch (err) {
      throw new DatabaseError(`SupabaseDatabase query failed: ${String(err)}`);
    }
  }

  async queryOne(sql: string, params: SqlValue[]): Promise<Row | null> {
    try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const rows = await this.sql.unsafe(sql, params as any[]);
      return rows.length > 0
        ? new MapRow(rows[0] as Record<string, SqlValue>)
        : null;
    } catch (err) {
      throw new DatabaseError(`SupabaseDatabase queryOne failed: ${String(err)}`);
    }
  }

  async batch(
    statements: Array<{ sql: string; params: SqlValue[] }>,
  ): Promise<WriteResult[]> {
    try {
      const results: WriteResult[] = [];
      await this.sql.begin(async (tx) => {
        for (const stmt of statements) {
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const result = await tx.unsafe(stmt.sql, stmt.params as any[]);
          results.push({ rowsAffected: result.count });
        }
      });
      return results;
    } catch (err) {
      throw new DatabaseError(`SupabaseDatabase batch failed: ${String(err)}`);
    }
  }
}
