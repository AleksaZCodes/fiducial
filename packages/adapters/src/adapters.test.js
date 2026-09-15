/**
 * Tests for @fiducial/adapters.
 * Imports from ../dist — built by tsc before this test runs (turbo: test dependsOn build).
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import {
  NoneDatabase,
  D1Database,
  DatabaseError,
  MapRow,
  NoneStorage,
  R2Storage,
  StorageError,
  NoneEmail,
  NoneDiagnostics,
  createNoneAdapters,
} from '../dist/index.js'

describe('@fiducial/adapters', () => {
  describe('NoneDatabase', () => {
    it('accepts env and ignores it', async () => {
      const db = new NoneDatabase({ DB: {} })
      assert.deepEqual(await db.execute('INSERT INTO t VALUES (?)', [1]), {
        rowsAffected: 0,
      })
    })

    it('query returns empty rows', async () => {
      const db = new NoneDatabase()
      assert.deepEqual(await db.query('SELECT * FROM t', []), [])
    })
  })

  describe('D1Database', () => {
    function fakeD1({ rows = [], first = null, changes = 0 } = {}) {
      const calls = []
      const stmt = {
        bind(...values) {
          calls.push(values)
          return stmt
        },
        async run() {
          return { success: true, meta: { changes } }
        },
        async all() {
          return { success: true, results: rows, meta: { changes } }
        },
        async first() {
          return first
        },
      }
      return {
        calls,
        prepare(sql) {
          calls.push(sql)
          return stmt
        },
        async batch(stmts) {
          return Promise.all(stmts.map((s) => s.run()))
        },
      }
    }

    it('throws when no DB binding is present', () => {
      assert.throws(() => new D1Database({}), DatabaseError)
    })

    it('execute binds params and reports changes from meta', async () => {
      const DB = fakeD1({ changes: 3 })
      const db = new D1Database({ DB })
      const result = await db.execute('DELETE FROM t WHERE id = ?', [1])
      assert.deepEqual(result, { rowsAffected: 3 })
    })

    it('query maps D1 results into Row objects', async () => {
      const DB = fakeD1({ rows: [{ id: 1, name: 'a' }] })
      const db = new D1Database({ DB })
      const [row] = await db.query('SELECT * FROM t', [])
      assert.equal(row.get('name'), 'a')
      assert.deepEqual(row.columns(), ['id', 'name'])
    })

    it('queryOne returns null when D1 finds no row', async () => {
      const DB = fakeD1({ first: null })
      const db = new D1Database({ DB })
      assert.equal(await db.queryOne('SELECT * FROM t WHERE id = ?', [1]), null)
    })

    it('queryOne wraps a found row', async () => {
      const DB = fakeD1({ first: { id: 1 } })
      const db = new D1Database({ DB })
      const row = await db.queryOne('SELECT * FROM t WHERE id = ?', [1])
      assert.equal(row.get('id'), 1)
    })

    it('batch returns one rowsAffected per statement', async () => {
      const DB = fakeD1({ changes: 1 })
      const db = new D1Database({ DB })
      const results = await db.batch([
        { sql: 'INSERT INTO a VALUES (?)', params: ['x'] },
        { sql: 'DELETE FROM b WHERE id = ?', params: [1] },
      ])
      assert.deepEqual(results, [{ rowsAffected: 1 }, { rowsAffected: 1 }])
    })

    it('wraps a thrown D1 error as DatabaseError', async () => {
      const DB = {
        prepare() {
          throw new Error('boom')
        },
      }
      const db = new D1Database({ DB })
      await assert.rejects(
        () => db.execute('SELECT 1', []),
        DatabaseError,
      )
    })
  })

  describe('MapRow', () => {
    it('get returns undefined for a missing column', () => {
      const row = new MapRow({ id: 1 })
      assert.equal(row.get('missing'), undefined)
    })
  })

  describe('NoneStorage', () => {
    it('accepts env and ignores it', async () => {
      const s = new NoneStorage({ BUCKET: {} })
      assert.equal(await s.get('missing'), null)
    })

    it('signedUrl returns an empty string', async () => {
      const s = new NoneStorage()
      assert.equal(await s.signedUrl('k', 60), '')
    })
  })

  describe('R2Storage', () => {
    function fakeR2({ objects = {}, pages } = {}) {
      return {
        objects,
        async put(key, bytes, options) {
          objects[key] = { bytes, options }
        },
        async get(key) {
          const obj = objects[key]
          if (!obj) return null
          return {
            async arrayBuffer() {
              return obj.bytes.buffer
            },
          }
        },
        async delete(key) {
          delete objects[key]
        },
        async list({ cursor } = {}) {
          const idx = cursor ? Number(cursor) : 0
          const page = pages[idx]
          return {
            objects: page.keys.map((key) => ({ key })),
            truncated: idx + 1 < pages.length,
            cursor: String(idx + 1),
          }
        },
      }
    }

    it('throws when no BUCKET binding is present', () => {
      assert.throws(() => new R2Storage({}), StorageError)
    })

    it('put stores bytes with content type', async () => {
      const BUCKET = fakeR2()
      const s = new R2Storage({ BUCKET })
      await s.put('a.txt', new Uint8Array([1, 2, 3]), 'text/plain')
      assert.deepEqual(BUCKET.objects['a.txt'].options, {
        httpMetadata: { contentType: 'text/plain' },
      })
    })

    it('get returns null for a missing key', async () => {
      const BUCKET = fakeR2()
      const s = new R2Storage({ BUCKET })
      assert.equal(await s.get('missing'), null)
    })

    it('get returns the stored bytes as Uint8Array', async () => {
      const BUCKET = fakeR2()
      const s = new R2Storage({ BUCKET })
      await s.put('a.bin', new Uint8Array([9, 8, 7]))
      const back = await s.get('a.bin')
      assert.deepEqual([...back], [9, 8, 7])
    })

    it('delete succeeds on a missing key', async () => {
      const BUCKET = fakeR2()
      const s = new R2Storage({ BUCKET })
      await s.delete('ghost')
    })

    it('list follows the cursor across pages', async () => {
      const BUCKET = fakeR2({
        pages: [{ keys: ['a', 'b'] }, { keys: ['c'] }],
      })
      const s = new R2Storage({ BUCKET })
      assert.deepEqual(await s.list(''), ['a', 'b', 'c'])
    })

    it('signedUrl throws, naming the reason', async () => {
      const BUCKET = fakeR2()
      const s = new R2Storage({ BUCKET })
      await assert.rejects(() => s.signedUrl('k', 60), (err) => {
        assert.ok(err instanceof StorageError)
        assert.match(err.message, /SigV4|not implemented/)
        return true
      })
    })
  })

  describe('createNoneAdapters', () => {
    it('builds a full no-op adapter set', () => {
      const adapters = createNoneAdapters()
      assert.ok(adapters.database instanceof NoneDatabase)
      assert.ok(adapters.storage instanceof NoneStorage)
      assert.ok(adapters.email instanceof NoneEmail)
      assert.ok(adapters.diagnostics instanceof NoneDiagnostics)
    })
  })
})
