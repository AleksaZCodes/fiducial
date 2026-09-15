/**
 * Database contract — relational storage: queries, migrations, transactions.
 *
 * Mirrors `fiducial_adapters::database` in Rust. Any method not shared by
 * D1, Supabase, Neon and Postgres stays on the vendor type, not here.
 */

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
