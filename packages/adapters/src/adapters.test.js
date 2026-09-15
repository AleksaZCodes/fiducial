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
  NoneBotProtection,
  Turnstile,
  BotProtectionError,
  NoneQueue,
  CloudflareQueue,
  QueueError,
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

  describe('NoneBotProtection', () => {
    it('always verifies successfully', async () => {
      const bp = new NoneBotProtection()
      assert.deepEqual(await bp.verify('any-token'), { success: true })
    })
  })

  describe('Turnstile', () => {
    it('throws when no secret key is present', () => {
      assert.throws(() => new Turnstile({}), BotProtectionError)
    })

    it('posts the token and secret, and maps a successful response', async () => {
      let capturedUrl
      let capturedBody
      const fakeFetch = async (url, init) => {
        capturedUrl = url
        capturedBody = init.body
        return {
          ok: true,
          async json() {
            return {
              success: true,
              challenge_ts: '2026-09-15T00:00:00Z',
              hostname: 'example.com',
            }
          },
        }
      }
      const t = new Turnstile({ TURNSTILE_SECRET_KEY: 'sekrit' }, fakeFetch)
      const outcome = await t.verify('tok-123', '203.0.113.1')

      assert.equal(
        capturedUrl,
        'https://challenges.cloudflare.com/turnstile/v0/siteverify',
      )
      assert.equal(capturedBody.get('secret'), 'sekrit')
      assert.equal(capturedBody.get('response'), 'tok-123')
      assert.equal(capturedBody.get('remoteip'), '203.0.113.1')
      assert.deepEqual(outcome, {
        success: true,
        challengeTs: '2026-09-15T00:00:00Z',
        hostname: 'example.com',
      })
    })

    it('maps a failed verification', async () => {
      const fakeFetch = async () => ({
        ok: true,
        async json() {
          return { success: false, ['error-codes']: ['invalid-input-response'] }
        },
      })
      const t = new Turnstile({ TURNSTILE_SECRET_KEY: 'sekrit' }, fakeFetch)
      const outcome = await t.verify('bad-token')
      assert.equal(outcome.success, false)
    })

    it('throws BotProtectionError on a non-OK HTTP response', async () => {
      const fakeFetch = async () => ({ ok: false, status: 503 })
      const t = new Turnstile({ TURNSTILE_SECRET_KEY: 'sekrit' }, fakeFetch)
      await assert.rejects(() => t.verify('tok'), BotProtectionError)
    })

    it('throws BotProtectionError when fetch itself rejects', async () => {
      const fakeFetch = async () => {
        throw new Error('network down')
      }
      const t = new Turnstile({ TURNSTILE_SECRET_KEY: 'sekrit' }, fakeFetch)
      await assert.rejects(() => t.verify('tok'), BotProtectionError)
    })
  })

  describe('NoneQueue', () => {
    it('send and sendBatch succeed silently', async () => {
      const q = new NoneQueue()
      await q.send(new Uint8Array([1]))
      await q.sendBatch([new Uint8Array([1]), new Uint8Array([2])])
    })
  })

  describe('CloudflareQueue', () => {
    function fakeQueue() {
      return { sent: [], batches: [],
        async send(message, options) {
          this.sent.push({ message, options })
        },
        async sendBatch(messages) {
          this.batches.push([...messages])
        },
      }
    }

    it('throws when no QUEUE binding is present', () => {
      assert.throws(() => new CloudflareQueue({}), QueueError)
    })

    it('send passes bytes through with contentType: bytes', async () => {
      const QUEUE = fakeQueue()
      const q = new CloudflareQueue({ QUEUE })
      const body = new Uint8Array([1, 2, 3])
      await q.send(body)
      assert.equal(QUEUE.sent.length, 1)
      assert.equal(QUEUE.sent[0].message, body)
      assert.deepEqual(QUEUE.sent[0].options, { contentType: 'bytes' })
    })

    it('sendBatch wraps every message the same way', async () => {
      const QUEUE = fakeQueue()
      const q = new CloudflareQueue({ QUEUE })
      await q.sendBatch([new Uint8Array([1]), new Uint8Array([2])])
      assert.equal(QUEUE.batches[0].length, 2)
      assert.equal(QUEUE.batches[0][0].contentType, 'bytes')
    })

    it('wraps a thrown send error as QueueError', async () => {
      const QUEUE = {
        async send() {
          throw new Error('boom')
        },
      }
      const q = new CloudflareQueue({ QUEUE })
      await assert.rejects(() => q.send(new Uint8Array([1])), QueueError)
    })
  })

  describe('createNoneAdapters', () => {
    it('builds a full no-op adapter set', () => {
      const adapters = createNoneAdapters()
      assert.ok(adapters.database instanceof NoneDatabase)
      assert.ok(adapters.storage instanceof NoneStorage)
      assert.ok(adapters.email instanceof NoneEmail)
      assert.ok(adapters.diagnostics instanceof NoneDiagnostics)
      assert.ok(adapters.botProtection instanceof NoneBotProtection)
      assert.ok(adapters.queue instanceof NoneQueue)
    })
  })
})
