/**
 * The audit log and the decision cache, against the REAL generated table.
 *
 * `migrations/0002_audit.sql` is produced by `fid derive` when `[identity]
 * audit = true`, and its `CHECK` on `reason` lists exactly the `Reason`
 * variants the rule can return. Testing against a hand-written copy of that
 * DDL would pass while the real one rejected every row — which is the failure
 * the identity work already hit once, with `GLOB` on Postgres.
 *
 * Runs under `test:schema` because it shells out to the real `fid`.
 */

import { describe, it, before } from 'node:test'
import assert from 'node:assert/strict'
import { DatabaseSync } from 'node:sqlite'
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync, mkdtempSync, existsSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

import {
  AuditLog,
  DecisionCache,
  explain,
  can,
} from '../dist/index.js'

const here = dirname(fileURLToPath(import.meta.url))
const repoRoot = join(here, '../../..')

const ALICE = { kind: 'user', id: 'a'.repeat(32) }
const BOB = { kind: 'user', id: 'b'.repeat(32) }
const R_DEV = { kind: 'device', id: 'dadadadadadadada' }

/** The audit schema as `fid derive` actually generates it. */
function generatedAuditSchema() {
  const fid = join(repoRoot, 'target/debug/fid')
  if (!existsSync(fid)) {
    throw new Error(
      `these tests run the real generated schema, and ${fid} is not built.\n` +
        'Run: cargo build -p fiducial-cli --bin fid',
    )
  }
  const dir = mkdtempSync(join(tmpdir(), 'audit-'))
  execFileSync(fid, ['new', 'p'], { cwd: dir, stdio: 'ignore' })
  const root = join(dir, 'p')
  execFileSync(fid, ['add', 'identity'], { cwd: root, stdio: 'ignore' })
  // Into the [identity] block, not onto the end of the file — appending put
  // `audit = true` under whatever section happened to be last, which was
  // [spine], and the key was silently somebody else's.
  const configPath = join(root, 'fiducial.toml')
  writeFileSync(
    configPath,
    readFileSync(configPath, 'utf8').replace('[identity]', '[identity]\naudit = true'),
  )
  execFileSync(fid, ['derive'], { cwd: root, stdio: 'ignore' })
  return readFileSync(join(root, 'migrations/0002_audit.sql'), 'utf8')
}

/** An `AuditDatabase` over real SQLite. */
function auditDb(schema) {
  const db = new DatabaseSync(':memory:')
  db.exec(schema)
  return {
    raw: db,
    async execute(sql, params) {
      const info = db.prepare(sql).run(...params)
      return { rowsAffected: Number(info.changes ?? 0) }
    },
    async query(sql, params) {
      return db
        .prepare(sql)
        .all(...params)
        .map((r) => ({ get: (c) => r[c] }))
    },
  }
}

describe('the audit log', () => {
  let SCHEMA
  before(() => {
    SCHEMA = generatedAuditSchema()
  })

  it('the generated DDL parses and creates the table', () => {
    const db = auditDb(SCHEMA)
    const tables = db.raw
      .prepare("SELECT name FROM sqlite_master WHERE type='table'")
      .all()
      .map((r) => r.name)
    assert.ok(tables.includes('grants_audit'), tables.join(', '))
  })

  it('records a decision with its reason, not just its verdict', async () => {
    const log = new AuditLog(auditDb(SCHEMA))
    const decision = await log.decide(BOB, 'read', R_DEV, [], Date.now())

    assert.equal(decision.allowed, false)
    assert.equal(decision.reason, 'no_grant')

    const [entry] = await log.forPrincipal(BOB)
    assert.equal(entry.allowed, false)
    assert.equal(entry.reason, 'no_grant')
    assert.deepEqual(entry.principal, BOB)
    assert.deepEqual(entry.resource, R_DEV)
    assert.equal(entry.action, 'read')
  })

  it('stores every reason the rule can actually return', async () => {
    // The CHECK on `reason` is the `Reason` variants. If the rule gains one
    // and the migration does not, this fails rather than losing the row.
    const db = auditDb(SCHEMA)
    const log = new AuditLog(db)
    const reasons = [
      'granted',
      'device_reading_itself',
      'not_identified',
      'no_grant',
      'role_too_weak',
      'expired',
      'no_clock',
      'delegator_lacks_authority',
      'delegation_too_deep',
    ]
    for (const reason of reasons) {
      await log.record(ALICE, 'read', R_DEV, { allowed: false, reason })
    }
    const rows = await log.forPrincipal(ALICE, 100)
    assert.equal(rows.length, reasons.length)
  })

  it('refuses a reason the rule cannot return', async () => {
    const log = new AuditLog(auditDb(SCHEMA))
    await assert.rejects(
      () => log.record(ALICE, 'read', R_DEV, { allowed: true, reason: 'vibes' }),
      /CHECK constraint failed/,
    )
  })

  it('allowed round-trips as a boolean, not as the integer SQLite stores', async () => {
    // SQLite has no boolean. A log that read `1` back as truthy-but-not-true
    // would be wrong in exactly the direction that matters.
    const log = new AuditLog(auditDb(SCHEMA))
    await log.record(ALICE, 'read', R_DEV, { allowed: true, reason: 'granted' })
    const [entry] = await log.forPrincipal(ALICE)
    assert.strictEqual(entry.allowed, true)
  })

  it('denials() finds refusals and skips grants', async () => {
    const log = new AuditLog(auditDb(SCHEMA))
    await log.record(ALICE, 'read', R_DEV, { allowed: true, reason: 'granted' })
    await log.record(BOB, 'admin', R_DEV, { allowed: false, reason: 'role_too_weak' })

    const denied = await log.denials()
    assert.equal(denied.length, 1)
    assert.deepEqual(denied[0].principal, BOB)
    assert.equal(denied[0].reason, 'role_too_weak')
  })

  it('rejects an injected table name', () => {
    assert.throws(() => new AuditLog(auditDb(SCHEMA), { table: 'a; DROP TABLE b' }))
  })
})

describe('the decision cache', () => {
  const grants = [{ principal: ALICE, resource: R_DEV, role: 'owner' }]

  it('a cached verdict matches an uncached one', () => {
    const cache = new DecisionCache()
    const direct = can(ALICE, 'write', R_DEV, grants, 1000)
    assert.equal(cache.can(ALICE, 'write', R_DEV, grants, 1000), direct)
    assert.equal(cache.can(ALICE, 'write', R_DEV, grants, 1000), direct)
  })

  it('invalidating drops every verdict and re-reads the table', () => {
    const cache = new DecisionCache()
    assert.equal(cache.can(ALICE, 'read', R_DEV, grants, 1000), true)
    cache.invalidate()
    assert.equal(cache.size, 0)
    assert.equal(cache.can(ALICE, 'read', R_DEV, [], 1000), false)
  })

  it('a different instant is a different question', () => {
    // The property that keeps the cache from outliving an expiry: `now` is
    // part of the key, so a later call cannot get an earlier answer.
    const expiring = [
      { principal: BOB, resource: R_DEV, role: 'owner', expiresAt: 1500 },
    ]
    const cache = new DecisionCache()
    assert.equal(cache.can(BOB, 'read', R_DEV, expiring, 1000), true)
    assert.equal(cache.can(BOB, 'read', R_DEV, expiring, 2000), false)
  })

  it('distinguishes actions and resources', () => {
    const viewer = [{ principal: ALICE, resource: R_DEV, role: 'viewer' }]
    const cache = new DecisionCache()
    assert.equal(cache.can(ALICE, 'read', R_DEV, viewer, 1), true)
    assert.equal(cache.can(ALICE, 'admin', R_DEV, viewer, 1), false)
  })

  it('a full cache evicts without ever answering wrongly', () => {
    const cache = new DecisionCache(2)
    for (let i = 0; i < 10; i++) {
      assert.equal(cache.can(ALICE, 'read', R_DEV, grants, i), true)
      assert.equal(cache.can(BOB, 'read', R_DEV, grants, i), false)
    }
    assert.ok(cache.size <= 2, `bounded: ${cache.size}`)
  })

  it('explain and can agree on every cached answer', () => {
    const cache = new DecisionCache()
    for (const action of ['read', 'write', 'admin']) {
      assert.equal(
        cache.can(ALICE, action, R_DEV, grants, 1),
        explain(ALICE, action, R_DEV, grants, 1).allowed,
      )
    }
  })
})
