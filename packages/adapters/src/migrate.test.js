/**
 * The migration runner, against real SQLite (`node:sqlite`) — which is what D1
 * runs.
 *
 * Mocking the database here would defeat the purpose. The three properties
 * under test — ordering, idempotency, drift — are all properties of what the
 * database ends up containing, and against a Map every one of them passes
 * whether or not the code is right.
 */

import { describe, it, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { DatabaseSync } from "node:sqlite";
import { createHash } from "node:crypto";

import { Migrator, MigrationError, DEFAULT_LEDGER } from "../dist/migrate.js";

/** A `Database` over real SQLite — the four methods the Migrator calls. */
function sqliteDb() {
  const db = new DatabaseSync(":memory:");
  return {
    raw: db,
    async execute(sql, params) {
      const info = db.prepare(sql).run(...params);
      return { rowsAffected: Number(info.changes ?? 0) };
    },
    async query(sql, params) {
      return db
        .prepare(sql)
        .all(...params)
        .map((r) => ({ get: (c) => r[c], columns: () => Object.keys(r) }));
    },
    async queryOne(sql, params) {
      const rows = await this.query(sql, params);
      return rows[0] ?? null;
    },
    async batch(statements) {
      // node:sqlite has no batch API; a transaction is the same guarantee and
      // is what the Postgres and Supabase adapters use.
      db.exec("BEGIN");
      try {
        const out = [];
        for (const s of statements) {
          out.push(await this.execute(s.sql, s.params));
        }
        db.exec("COMMIT");
        return out;
      } catch (e) {
        db.exec("ROLLBACK");
        throw e;
      }
    },
  };
}

// A real SHA-256, matching what `fid derive` writes into the manifest.
//
// The first version of this helper was a cheap stand-in built from the SQL's
// length and first few characters — and two different migrations in these very
// tests hashed the same, so drift went undetected and the tests blamed the
// runner. A fake hash is not a hash.
const hashOf = (s) => createHash("sha256").update(s).digest("hex");
const mig = (id, sql, file = `${id}.sql`) => ({ id, file, hash: hashOf(sql), sql });

const ONE = mig("0001_users", "CREATE TABLE users (id TEXT PRIMARY KEY)");
const TWO = mig("0002_email", "ALTER TABLE users ADD COLUMN email TEXT");

describe("ordering", () => {
  let db;
  beforeEach(() => {
    db = sqliteDb();
  });

  it("applies migrations in list order", async () => {
    // 0002 references what 0001 created, so the wrong order is not a style
    // question — it is an error from SQLite.
    const report = await new Migrator(db, [ONE, TWO]).apply();
    assert.deepEqual(
      report.map((r) => r.id),
      ["0001_users", "0002_email"],
    );
    const cols = db.raw
      .prepare("SELECT name FROM pragma_table_info('users')")
      .all()
      .map((r) => r.name);
    assert.deepEqual(cols.sort(), ["email", "id"]);
  });

  it("stops at the first failure, keeping what already applied", async () => {
    const broken = mig("0002_broken", "ALTER TABLE nope ADD COLUMN x TEXT");
    const m = new Migrator(db, [ONE, broken, TWO]);
    await assert.rejects(() => m.apply(), MigrationError);

    // 0001 applied and is recorded; the rest did not run. A later migration
    // may depend on the one that failed, so continuing is never right.
    const done = await m.ledgerEntries();
    assert.deepEqual([...done.keys()], ["0001_users"]);
  });

  it("rejects a duplicate id rather than silently applying one of them", () => {
    assert.throws(() => new Migrator(db, [ONE, { ...TWO, id: ONE.id }]), MigrationError);
  });
});

describe("idempotency", () => {
  let db;
  beforeEach(() => {
    db = sqliteDb();
  });

  it("a second apply() skips everything and changes nothing", async () => {
    const m = new Migrator(db, [ONE, TWO]);
    await m.apply();

    // The real scenario: a deploy re-runs. Applying 0002 twice is a duplicate
    // column error, so "we only deploy once" is the only thing protecting a
    // runner without a ledger, and that is not a property any system has.
    const second = await m.apply();
    assert.deepEqual(
      second.map((r) => r.outcome),
      ["skipped", "skipped"],
    );
  });

  it("applies only what is new when a migration is added later", async () => {
    await new Migrator(db, [ONE]).apply();

    const report = await new Migrator(db, [ONE, TWO]).apply();
    assert.deepEqual(report, [
      { id: "0001_users", outcome: "skipped" },
      { id: "0002_email", outcome: "applied" },
    ]);
  });

  it("records each migration in the same batch that applies it", async () => {
    // A migration that ran without being recorded re-runs on the next deploy.
    const failing = {
      ...sqliteDb(),
      batch: async () => {
        throw new Error("connection lost");
      },
    };
    const m = new Migrator(failing, [ONE]);
    await assert.rejects(() => m.apply(), MigrationError);
    assert.equal((await m.ledgerEntries()).size, 0);
  });

  it("creates the ledger table on first use and tolerates it existing", async () => {
    const m = new Migrator(db, [ONE]);
    await m.ensureLedger();
    await m.ensureLedger();
    const tables = db.raw
      .prepare("SELECT name FROM sqlite_master WHERE type='table'")
      .all()
      .map((r) => r.name);
    assert.ok(tables.includes(DEFAULT_LEDGER), tables.join(", "));
  });
});

describe("drift", () => {
  let db;
  beforeEach(() => {
    db = sqliteDb();
  });

  it("detects a migration edited after it was applied", async () => {
    await new Migrator(db, [ONE]).apply();

    // Same id, different text. Every environment that already applied this has
    // the old schema; every new one gets the new schema; nothing either of
    // them can see says they disagree.
    const edited = mig("0001_users", "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT)");
    const drift = await new Migrator(db, [edited]).drift();

    assert.equal(drift.length, 1);
    assert.equal(drift[0].id, "0001_users");
    assert.equal(drift[0].applied, ONE.hash);
    assert.equal(drift[0].current, edited.hash);
  });

  it("refuses to apply anything while an applied migration has drifted", async () => {
    await new Migrator(db, [ONE]).apply();
    const edited = mig("0001_users", "CREATE TABLE users (id TEXT, extra TEXT)");

    const m = new Migrator(db, [edited, TWO]);
    await assert.rejects(
      () => m.apply(),
      (e) => e instanceof MigrationError && /changed after they were applied/.test(e.message),
    );

    // 0002 did not sneak in on top of a schema nobody has reconciled.
    assert.equal((await m.ledgerEntries()).has("0002_email"), false);
  });

  it("status reports drift instead of throwing", async () => {
    await new Migrator(db, [ONE]).apply();
    const edited = mig("0001_users", "CREATE TABLE users (id TEXT, extra TEXT)");

    // `status` is what you run to find out you have a problem, so it cannot be
    // the thing that refuses to tell you.
    const { pending, drift } = await new Migrator(db, [edited, TWO]).status();
    assert.equal(drift.length, 1);
    assert.deepEqual(
      pending.map((p) => p.id),
      ["0002_email"],
    );
  });

  it("an unedited migration does not read as drift", async () => {
    const m = new Migrator(db, [ONE, TWO]);
    await m.apply();
    assert.deepEqual(await m.drift(), []);
  });
});

describe("dialect", () => {
  it("uses positional placeholders on postgres", async () => {
    const seen = [];
    const spy = {
      execute: async () => ({ rowsAffected: 0 }),
      query: async () => [],
      queryOne: async () => null,
      batch: async (statements) => {
        seen.push(...statements.map((s) => s.sql));
        return statements.map(() => ({ rowsAffected: 0 }));
      },
    };
    await new Migrator(spy, [ONE], { dialect: "postgres" }).apply();

    // The same fact `src/identity.generated.ts` already carries: a Postgres
    // table queried with SQLite's `?` is a syntax error on every statement,
    // and it is silent until runtime.
    const insert = seen.find((s) => s.includes("INSERT INTO"));
    assert.match(insert, /\$1, \$2, \$3/);
    assert.ok(!insert.includes("?"), insert);
  });
});
