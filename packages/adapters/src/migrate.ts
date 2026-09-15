/**
 * Schema migrations — ordering, idempotency, and drift detection.
 *
 * `fid derive` generates `src/migrations.generated.ts` from `migrations/*.sql`.
 * This applies it through the `Database` contract, so it works on D1 today and
 * on any future vendor for free — a migration runner that knew about D1 would
 * be the vendor lock-in the adapter system exists to prevent.
 *
 * What grant storage generated was the first table, and its spec said plainly
 * that this was missing:
 *
 * > This is not a migrations system. It generates the first table. The moment
 * > the schema changes you need ordering, idempotency and drift detection
 * > against a live database.
 *
 * Each of the three, and why none is optional:
 *
 * - **Ordering.** `0002` may reference what `0001` created.
 * - **Idempotency.** Deploys re-run. Applying a migration twice is at best an
 *   error and at worst a duplicate column.
 * - **Drift.** A migration edited *after* it was applied is the dangerous one:
 *   environments that already ran it keep the old schema, new ones get the new
 *   schema, and neither can tell it is wrong. This is why the ledger stores a
 *   hash and not just an id.
 */

import type { Database, SqlValue } from "./database.js";

/** One migration, as `fid derive` emits it. */
export interface Migration {
  /** Stable identity: `0001_grants`. Survives a change of file extension. */
  readonly id: string;
  /** Source file name, for error messages. */
  readonly file: string;
  /** SHA-256 of the SQL, as it stood when generated. */
  readonly hash: string;
  /** The SQL to apply. */
  readonly sql: string;
}

/** What happened to one migration during `apply()`. */
export interface Applied {
  readonly id: string;
  /** `applied` — it ran now. `skipped` — the ledger already had it. */
  readonly outcome: "applied" | "skipped";
}

/** A migration whose recorded hash no longer matches the file. */
export interface Drift {
  readonly id: string;
  readonly file: string;
  /** The hash recorded when it was applied. */
  readonly applied: string;
  /** The hash of the migration as it stands now. */
  readonly current: string;
}

export class MigrationError extends Error {
  constructor(
    message: string,
    readonly id?: string,
  ) {
    super(message);
    this.name = "MigrationError";
  }
}

/** The default ledger table name — mirrors `schema::LEDGER` in Rust. */
export const DEFAULT_LEDGER = "_fiducial_migrations";

export interface MigratorOptions {
  /** Ledger table name. Defaults to `_fiducial_migrations`. */
  readonly ledgerTable?: string;
  /**
   * Bound-parameter style. `?` for SQLite/D1, `$n` for Postgres.
   *
   * The same fact `src/identity.generated.ts` already carries, and for the
   * same reason: a Postgres table queried with SQLite placeholders is a syntax
   * error on every statement, and it is silent until runtime.
   */
  readonly dialect?: "sqlite" | "postgres";
}

export class Migrator {
  private readonly ledger: string;
  private readonly dialect: "sqlite" | "postgres";

  constructor(
    private readonly db: Database,
    private readonly migrations: readonly Migration[],
    options: MigratorOptions = {},
  ) {
    this.ledger = options.ledgerTable ?? DEFAULT_LEDGER;
    this.dialect = options.dialect ?? "sqlite";

    // A duplicate id means the generated manifest is wrong, and the failure it
    // would otherwise cause — one of the two silently never applying — is
    // invisible. `fid derive` rejects this too; checking again here is cheap
    // and this class can be handed a hand-built list.
    const seen = new Set<string>();
    for (const m of migrations) {
      if (seen.has(m.id)) {
        throw new MigrationError(`duplicate migration id \`${m.id}\``, m.id);
      }
      seen.add(m.id);
    }
  }

  /** `?` or `$1`, `$2`, … depending on dialect. */
  private placeholder(index: number): string {
    return this.dialect === "postgres" ? `$${index}` : "?";
  }

  /**
   * Create the ledger if it is not there.
   *
   * `IF NOT EXISTS` rather than a catch: the first deploy and every subsequent
   * one run the same statement, which is the whole idea.
   */
  async ensureLedger(): Promise<void> {
    const timestamp = this.dialect === "postgres" ? "TIMESTAMPTZ" : "TEXT";
    await this.db.execute(
      `CREATE TABLE IF NOT EXISTS ${this.ledger} (
  id          TEXT PRIMARY KEY,
  hash        TEXT NOT NULL,
  applied_at  ${timestamp} NOT NULL
)`,
      [],
    );
  }

  /** Every migration the database has a record of, id → hash. */
  async ledgerEntries(): Promise<Map<string, string>> {
    await this.ensureLedger();
    const rows = await this.db.query(
      `SELECT id, hash FROM ${this.ledger}`,
      [],
    );
    const out = new Map<string, string>();
    for (const row of rows) {
      const id = row.get("id");
      const hash = row.get("hash");
      if (typeof id === "string" && typeof hash === "string") {
        out.set(id, hash);
      }
    }
    return out;
  }

  /**
   * Migrations that were applied and have since been edited.
   *
   * Reported rather than repaired. There is no safe automatic answer: the
   * database already has whatever the old text did, and re-running the new
   * text may or may not be valid against it. A person has to decide, and the
   * only useful thing this can do is make sure they know.
   */
  async drift(): Promise<Drift[]> {
    const entries = await this.ledgerEntries();
    const out: Drift[] = [];
    for (const m of this.migrations) {
      const applied = entries.get(m.id);
      if (applied !== undefined && applied !== m.hash) {
        out.push({ id: m.id, file: m.file, applied, current: m.hash });
      }
    }
    return out;
  }

  /** Migrations not yet in the ledger, in order. */
  async pending(): Promise<Migration[]> {
    const entries = await this.ledgerEntries();
    return this.migrations.filter((m) => !entries.has(m.id));
  }

  /**
   * Apply every pending migration, in order.
   *
   * Refuses to run at all if any already-applied migration has drifted. The
   * database is in a state the code no longer describes, and applying more on
   * top of it compounds a problem nobody has looked at yet.
   */
  async apply(): Promise<Applied[]> {
    const drifted = await this.drift();
    if (drifted.length > 0) {
      const lines = drifted
        .map(
          (d) =>
            `  ${d.file}: applied as ${d.applied.slice(0, 8)}, now ${d.current.slice(0, 8)}`,
        )
        .join("\n");
      throw new MigrationError(
        `${drifted.length} migration(s) changed after they were applied:\n${lines}\n\n` +
          "This database ran the old text. Every environment that has not yet " +
          "applied it will run the new text,\nand neither can tell it disagrees " +
          "with the other. Add a new migration rather than editing an applied " +
          "one.",
      );
    }

    const entries = await this.ledgerEntries();
    const results: Applied[] = [];

    for (const m of this.migrations) {
      if (entries.has(m.id)) {
        results.push({ id: m.id, outcome: "skipped" });
        continue;
      }
      // The migration and its ledger row go together as one batch, so a
      // migration that ran without being recorded — which would re-run on the
      // next deploy — is not a state this can reach where the vendor's batch
      // is atomic.
      try {
        await this.db.batch([
          { sql: m.sql, params: [] },
          {
            sql: `INSERT INTO ${this.ledger} (id, hash, applied_at) VALUES (${this.placeholder(
              1,
            )}, ${this.placeholder(2)}, ${this.placeholder(3)})`,
            params: [m.id, m.hash, new Date().toISOString()] as SqlValue[],
          },
        ]);
      } catch (cause) {
        // Ordering means a later migration may depend on this one, so stopping
        // is the only correct response. What already applied stays applied and
        // is recorded, so re-running resumes from here.
        throw new MigrationError(
          `${m.file} failed: ${cause instanceof Error ? cause.message : String(cause)}`,
          m.id,
        );
      }
      results.push({ id: m.id, outcome: "applied" });
    }

    return results;
  }

  /**
   * What `apply()` would do, without doing it.
   *
   * Drift is returned rather than thrown here: `status` is what you run to
   * find out you have a problem, so it cannot be the thing that refuses to
   * tell you.
   */
  async status(): Promise<{ pending: Migration[]; drift: Drift[] }> {
    return { pending: await this.pending(), drift: await this.drift() };
  }
}
