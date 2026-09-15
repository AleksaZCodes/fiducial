/**
 * Tests for grant storage.
 *
 * `SqlGrantStore` is exercised against **real SQLite** (`node:sqlite`), using
 * the **real generated schema** read off disk — not a hand-written copy and
 * not a fake database. D1 is SQLite, so this is the same engine and the same
 * DDL a deployed product runs.
 *
 * That matters because the failure mode of a mocked store is that every query
 * "works": a typo'd column, a broken `ON CONFLICT`, a `CHECK` that rejects
 * nothing — all pass against a Map. Here they fail.
 */

import { describe, it, before } from 'node:test'
import assert from 'node:assert/strict'
import { DatabaseSync } from 'node:sqlite'
import { execFileSync } from 'node:child_process'
import { readFileSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  SqlGrantStore,
  MemoryGrantStore,
  can,
  effectiveRole,
  ANONYMOUS,
} from '../dist/index.js'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(here, '../../..')

const ALICE = { kind: 'user', id: 'a'.repeat(32) }
const BOB = { kind: 'user', id: 'b'.repeat(32) }
const DEV = { kind: 'device', id: 'dadadadadadadada' }
const DEV2 = { kind: 'device', id: 'dbdbdbdbdbdbdbdb' }
const R_DEV = { kind: 'device', id: 'dadadadadadadada' }
const R_DEV2 = { kind: 'device', id: 'dbdbdbdbdbdbdbdb' }
const PLATFORM = { kind: 'platform' }

/**
 * The schema as `fid derive` actually generates it.
 *
 * Generated fresh from the real binary rather than checked in here — a copy
 * would be the second declaration this whole pipeline exists to delete, and
 * it would go stale the first time the identity model changed.
 */
function generatedSchema() {
  const fid = join(repoRoot, 'target/debug/fid')
  const dir = mkdtempSync(join(tmpdir(), 'grants-'))
  execFileSync(fid, ['new', 'p'], { cwd: dir, stdio: 'ignore' })
  const root = join(dir, 'p')
  execFileSync(fid, ['add', 'identity'], { cwd: root, stdio: 'ignore' })
  execFileSync(fid, ['derive'], { cwd: root, stdio: 'ignore' })
  return readFileSync(join(root, 'migrations/0001_grants.sql'), 'utf8')
}

/** A `GrantDatabase` over real SQLite — the two methods the store calls. */
function sqliteDb(schema) {
  const db = new DatabaseSync(':memory:')
  db.exec(schema)
  return {
    raw: db,
    async query(sql, params) {
      const rows = db.prepare(sql).all(...params)
      return rows.map((r) => ({ get: (column) => r[column] }))
    },
    async execute(sql, params) {
      const info = db.prepare(sql).run(...params)
      return { rowsAffected: Number(info.changes ?? 0) }
    },
  }
}

let SCHEMA
before(() => {
  SCHEMA = generatedSchema()
})

describe('grant storage', () => {
  describe('the generated schema', () => {
    it('is valid SQLite that the real binary produced', () => {
      const db = new DatabaseSync(':memory:')
      db.exec(SCHEMA) // throws if the generated DDL does not parse
      assert.match(SCHEMA, /CREATE TABLE IF NOT EXISTS grants/)
    })

    it('lists exactly the roles the model defines', () => {
      assert.match(SCHEMA, /role.*CHECK.*'viewer', 'member', 'admin', 'owner'/s)
    })

    /**
     * The rule `can()` enforces at runtime, enforced by the database too: a
     * grant that could never be honoured cannot even be written.
     */
    it('refuses to store a grant to anonymous', () => {
      const db = sqliteDb(SCHEMA)
      assert.throws(
        () =>
          db.raw
            .prepare(
              `INSERT INTO grants VALUES ('anonymous','x','device','d','owner','now')`,
            )
            .run(),
        /CHECK constraint failed/,
      )
    })

    it('refuses to store a grant to an all-zero sentinel id', () => {
      const db = sqliteDb(SCHEMA)
      assert.throws(
        () =>
          db.raw
            .prepare(
              `INSERT INTO grants VALUES ('device','0000000000000000','device','d','owner','now')`,
            )
            .run(),
        /CHECK constraint failed/,
      )
    })

    it('refuses an unknown role', () => {
      const db = sqliteDb(SCHEMA)
      assert.throws(
        () =>
          db.raw
            .prepare(
              `INSERT INTO grants VALUES ('user','alice','device','d','superuser','now')`,
            )
            .run(),
        /CHECK constraint failed/,
      )
    })
  })

  // The same suite runs against both stores. They are two implementations of
  // one interface, and a difference between them is a bug in whichever one a
  // product is not using in production.
  for (const [name, make] of [
    ['SqlGrantStore (real SQLite)', () => new SqlGrantStore(sqliteDb(SCHEMA))],
    ['MemoryGrantStore', () => new MemoryGrantStore()],
  ]) {
    describe(name, () => {
      it('starts empty — deny by default is not a special case', async () => {
        const store = make()
        assert.deepEqual(await store.grantsFor(ALICE), [])
        assert.equal(can(ALICE, 'read', R_DEV, await store.grantsFor(ALICE)), false)
      })

      it('a stored grant is the one can() reads', async () => {
        const store = make()
        await store.grant(ALICE, R_DEV, 'owner')

        const grants = await store.grantsFor(ALICE)
        assert.equal(grants.length, 1)
        assert.equal(can(ALICE, 'admin', R_DEV, grants), true)
        assert.equal(effectiveRole(ALICE, R_DEV, grants), 'owner')
      })

      it('a grant reaches only its own principal', async () => {
        const store = make()
        await store.grant(ALICE, R_DEV, 'owner')
        assert.deepEqual(await store.grantsFor(BOB), [])
        assert.equal(can(BOB, 'read', R_DEV, await store.grantsFor(BOB)), false)
      })

      it('a grant reaches only its own resource', async () => {
        const store = make()
        await store.grant(ALICE, R_DEV, 'owner')
        const grants = await store.grantsFor(ALICE)
        assert.equal(can(ALICE, 'read', R_DEV2, grants), false)
      })

      it('sharing: a viewer may read but not write', async () => {
        const store = make()
        await store.grant(ALICE, R_DEV, 'owner')
        await store.grant(BOB, R_DEV, 'viewer')

        const bobs = await store.grantsFor(BOB)
        assert.equal(can(BOB, 'read', R_DEV, bobs), true)
        assert.equal(can(BOB, 'write', R_DEV, bobs), false)
      })

      it('granting again replaces the role rather than adding a second', async () => {
        const store = make()
        await store.grant(BOB, R_DEV, 'viewer')
        await store.grant(BOB, R_DEV, 'member')

        const grants = await store.grantsFor(BOB)
        assert.equal(grants.length, 1, 'one role per principal per resource')
        assert.equal(effectiveRole(BOB, R_DEV, grants), 'member')
      })

      it('revoke removes access', async () => {
        const store = make()
        await store.grant(BOB, R_DEV, 'member')
        await store.revoke(BOB, R_DEV)

        const grants = await store.grantsFor(BOB)
        assert.equal(can(BOB, 'read', R_DEV, grants), false)
      })

      it('revoking something never granted is not an error', async () => {
        const store = make()
        await store.revoke(BOB, R_DEV)
      })

      it('a platform grant is returned and covers every resource', async () => {
        const store = make()
        const svc = { kind: 'service', id: '5c5c5c5c5c5c5c5c' }
        await store.grant(svc, PLATFORM, 'admin')

        const grants = await store.grantsFor(svc)
        assert.equal(can(svc, 'admin', R_DEV, grants), true)
        assert.equal(can(svc, 'admin', R_DEV2, grants), true)
      })

      it('grantsOn answers the sharing question grantsFor cannot', async () => {
        const store = make()
        await store.grant(ALICE, R_DEV, 'owner')
        await store.grant(BOB, R_DEV, 'viewer')
        await store.grant(ALICE, R_DEV2, 'owner')

        const onDev = await store.grantsOn(R_DEV)
        assert.equal(onDev.length, 2)
        assert.deepEqual(
          onDev.map((g) => g.role).sort(),
          ['owner', 'viewer'],
        )
      })

      it('a device principal can hold a grant, like any other', async () => {
        const store = make()
        await store.grant(DEV, R_DEV2, 'member')
        const grants = await store.grantsFor(DEV)
        assert.equal(can(DEV, 'write', R_DEV2, grants), true)
      })

      it('anonymous holds nothing and is never queried for', async () => {
        const store = make()
        assert.deepEqual(await store.grantsFor(ANONYMOUS), [])
      })

      it('granting to anonymous is refused, not silently stored', async () => {
        const store = make()
        await assert.rejects(() => store.grant(ANONYMOUS, R_DEV, 'owner'))
      })
    })
  }

  describe('SqlGrantStore specifics', () => {
    it('rejects a table name that is not a plain identifier', () => {
      const db = sqliteDb(SCHEMA)
      assert.throws(() => new SqlGrantStore(db, 'grants; DROP TABLE users'))
      assert.throws(() => new SqlGrantStore(db, ''))
    })

    it('accepts a custom table name', () => {
      const db = sqliteDb(SCHEMA.replaceAll('grants', 'access_grants'))
      const store = new SqlGrantStore(db, 'access_grants')
      assert.ok(store)
    })

    it('writes the row the schema expects — no column drift', async () => {
      const db = sqliteDb(SCHEMA)
      const store = new SqlGrantStore(db)
      await store.grant(ALICE, R_DEV, 'owner')

      const [row] = db.raw.prepare('SELECT * FROM grants').all()
      assert.equal(row.principal_kind, 'user')
      assert.equal(row.principal_id, ALICE.id)
      assert.equal(row.resource_kind, 'device')
      assert.equal(row.resource_id, R_DEV.id)
      assert.equal(row.role, 'owner')
      assert.ok(row.granted_at, 'granted_at is recorded')
    })

    it('stores a platform grant with an empty resource id', async () => {
      const db = sqliteDb(SCHEMA)
      const store = new SqlGrantStore(db)
      const svc = { kind: 'service', id: '5c5c5c5c5c5c5c5c' }
      await store.grant(svc, PLATFORM, 'admin')

      const [row] = db.raw.prepare('SELECT * FROM grants').all()
      assert.equal(row.resource_kind, 'platform')
      assert.equal(row.resource_id, '')
    })
  })
})
