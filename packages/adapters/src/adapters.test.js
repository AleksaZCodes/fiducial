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
  NoneAuth,
  AuthError,
  CookieKeyValueStore,
  BearerKeyValueStore,
  SupabaseAuth,
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

  describe('NoneAuth', () => {
    it('signUp, signIn, signInWithOAuth, exchangeCodeForSession, resetPasswordForEmail, updatePassword all throw', async () => {
      const auth = new NoneAuth()
      await assert.rejects(() => auth.signUp('a@example.com', 'pw'), AuthError)
      await assert.rejects(() => auth.signIn('a@example.com', 'pw'), AuthError)
      await assert.rejects(() => auth.signInWithOAuth('google', 'https://x'), AuthError)
      await assert.rejects(() => auth.exchangeCodeForSession('code'), AuthError)
      await assert.rejects(
        () => auth.resetPasswordForEmail('a@example.com', 'https://x'),
        AuthError,
      )
      await assert.rejects(() => auth.updatePassword('new-pw'), AuthError)
    })

    it('signOut succeeds silently and getSession reports signed out', async () => {
      const auth = new NoneAuth()
      await auth.signOut()
      assert.equal(await auth.getSession(), null)
    })
  })

  describe('CookieKeyValueStore', () => {
    function fakeCookies() {
      const jar = new Map()
      return {
        jar,
        get(name) {
          return jar.get(name)
        },
        set(name, value, options) {
          jar.set(name, value)
          this.lastOptions = options
        },
        delete(name) {
          jar.delete(name)
        },
      }
    }

    it('getItem reads from the cookie jar, or null when absent', () => {
      const cookies = fakeCookies()
      const store = new CookieKeyValueStore(cookies)
      assert.equal(store.getItem('missing'), null)
      cookies.jar.set('k', 'v')
      assert.equal(store.getItem('k'), 'v')
    })

    it('setItem writes through with httpOnly/secure defaults', () => {
      const cookies = fakeCookies()
      const store = new CookieKeyValueStore(cookies)
      store.setItem('k', 'v')
      assert.equal(cookies.jar.get('k'), 'v')
      assert.equal(cookies.lastOptions.httpOnly, true)
      assert.equal(cookies.lastOptions.secure, true)
    })

    it('removeItem deletes from the jar', () => {
      const cookies = fakeCookies()
      const store = new CookieKeyValueStore(cookies)
      cookies.jar.set('k', 'v')
      store.removeItem('k')
      assert.ok(!cookies.jar.has('k'))
    })
  })

  describe('BearerKeyValueStore', () => {
    it('fromAuthorizationHeader extracts the bearer token', () => {
      const store = BearerKeyValueStore.fromAuthorizationHeader('Bearer abc123')
      assert.equal(store.getItem('sb-access-token'), 'abc123')
    })

    it('fromAuthorizationHeader with no header is empty', () => {
      const store = BearerKeyValueStore.fromAuthorizationHeader(null)
      assert.equal(store.getItem('sb-access-token'), null)
    })

    it('setItem/getItem/removeItem round-trip in memory', () => {
      const store = new BearerKeyValueStore()
      store.setItem('k', 'v')
      assert.equal(store.getItem('k'), 'v')
      store.removeItem('k')
      assert.equal(store.getItem('k'), null)
    })
  })

  describe('SupabaseAuth', () => {
    function fakeSupabaseClient(overrides = {}) {
      return {
        auth: {
          async signUp() {
            return { data: { session: fakeSession(), user: fakeUser() }, error: null }
          },
          async signInWithPassword() {
            return { data: { session: fakeSession(), user: fakeUser() }, error: null }
          },
          async signInWithOAuth() {
            return { data: { url: 'https://provider.example/authorize' }, error: null }
          },
          async exchangeCodeForSession() {
            return { data: { session: fakeSession(), user: fakeUser() }, error: null }
          },
          async signOut() {
            return { error: null }
          },
          async getSession() {
            return { data: { session: fakeSession() }, error: null }
          },
          async resetPasswordForEmail() {
            return { error: null }
          },
          async updateUser() {
            return { error: null }
          },
          ...overrides,
        },
      }
    }

    function fakeUser() {
      return {
        id: 'user-1',
        email: 'a@example.com',
        email_confirmed_at: '2026-01-01T00:00:00Z',
        created_at: '2026-01-01T00:00:00Z',
      }
    }

    function fakeSession() {
      return {
        access_token: 'access-tok',
        refresh_token: 'refresh-tok',
        expires_at: 9999999999,
        user: fakeUser(),
      }
    }

    function env() {
      return { SUPABASE_URL: 'https://x.supabase.co', SUPABASE_ANON_KEY: 'anon-key' }
    }

    function cookieStore() {
      const jar = new Map()
      return new CookieKeyValueStore({
        get: (n) => jar.get(n),
        set: (n, v) => jar.set(n, v),
        delete: (n) => jar.delete(n),
      })
    }

    it('throws when env is missing SUPABASE_URL/SUPABASE_ANON_KEY', () => {
      assert.throws(() => new SupabaseAuth({}, cookieStore(), fakeSupabaseClient), AuthError)
    })

    it('signUp maps the returned session', async () => {
      const auth = new SupabaseAuth(env(), cookieStore(), () => fakeSupabaseClient())
      const session = await auth.signUp('a@example.com', 'pw')
      assert.equal(session.accessToken, 'access-tok')
      assert.equal(session.user.email, 'a@example.com')
      assert.equal(session.user.emailVerified, true)
    })

    it('signUp throws when email confirmation is pending (no session)', async () => {
      const client = fakeSupabaseClient({
        async signUp() {
          return { data: { session: null, user: fakeUser() }, error: null }
        },
      })
      const auth = new SupabaseAuth(env(), cookieStore(), () => client)
      await assert.rejects(() => auth.signUp('a@example.com', 'pw'), AuthError)
    })

    it('signIn maps the returned session', async () => {
      const auth = new SupabaseAuth(env(), cookieStore(), () => fakeSupabaseClient())
      const session = await auth.signIn('a@example.com', 'pw')
      assert.equal(session.refreshToken, 'refresh-tok')
    })

    it('signIn with invalid credentials maps to a clear AuthError', async () => {
      const client = fakeSupabaseClient({
        async signInWithPassword() {
          return {
            data: { session: null, user: null },
            error: { message: 'Invalid login credentials', status: 400 },
          }
        },
      })
      const auth = new SupabaseAuth(env(), cookieStore(), () => client)
      await assert.rejects(() => auth.signIn('a@example.com', 'wrong'), (err) => {
        assert.ok(err instanceof AuthError)
        assert.match(err.message, /invalid credentials/)
        return true
      })
    })

    it('signIn rate-limited maps to a clear AuthError', async () => {
      const client = fakeSupabaseClient({
        async signInWithPassword() {
          return {
            data: { session: null, user: null },
            error: { message: 'too many requests', status: 429 },
          }
        },
      })
      const auth = new SupabaseAuth(env(), cookieStore(), () => client)
      await assert.rejects(() => auth.signIn('a@example.com', 'pw'), /rate limited/)
    })

    it('signInWithOAuth returns the redirect URL under a cookie store', async () => {
      const auth = new SupabaseAuth(env(), cookieStore(), () => fakeSupabaseClient())
      const { url } = await auth.signInWithOAuth('google', 'https://app.example/callback')
      assert.equal(url, 'https://provider.example/authorize')
    })

    it('signInWithOAuth throws under a bearer store', async () => {
      const auth = new SupabaseAuth(
        env(),
        new BearerKeyValueStore(),
        () => fakeSupabaseClient(),
      )
      await assert.rejects(
        () => auth.signInWithOAuth('google', 'https://app.example/callback'),
        AuthError,
      )
    })

    it('exchangeCodeForSession maps the session under a cookie store', async () => {
      const auth = new SupabaseAuth(env(), cookieStore(), () => fakeSupabaseClient())
      const session = await auth.exchangeCodeForSession('code-123')
      assert.equal(session.accessToken, 'access-tok')
    })

    it('exchangeCodeForSession throws under a bearer store', async () => {
      const auth = new SupabaseAuth(
        env(),
        new BearerKeyValueStore(),
        () => fakeSupabaseClient(),
      )
      await assert.rejects(() => auth.exchangeCodeForSession('code-123'), AuthError)
    })

    it('signOut calls through and resolves', async () => {
      const auth = new SupabaseAuth(env(), cookieStore(), () => fakeSupabaseClient())
      await auth.signOut()
    })

    it('getSession returns null when the client reports none', async () => {
      const client = fakeSupabaseClient({
        async getSession() {
          return { data: { session: null }, error: null }
        },
      })
      const auth = new SupabaseAuth(env(), cookieStore(), () => client)
      assert.equal(await auth.getSession(), null)
    })

    it('getSession maps a present session', async () => {
      const auth = new SupabaseAuth(env(), cookieStore(), () => fakeSupabaseClient())
      const session = await auth.getSession()
      assert.equal(session.user.id, 'user-1')
    })

    it('resetPasswordForEmail calls through', async () => {
      const auth = new SupabaseAuth(env(), cookieStore(), () => fakeSupabaseClient())
      await auth.resetPasswordForEmail('a@example.com', 'https://app.example/reset')
    })

    it('updatePassword calls through', async () => {
      const auth = new SupabaseAuth(env(), cookieStore(), () => fakeSupabaseClient())
      await auth.updatePassword('new-password')
    })

    it('propagates a generic vendor error', async () => {
      const client = fakeSupabaseClient({
        async signOut() {
          return { error: { message: 'network blip' } }
        },
      })
      const auth = new SupabaseAuth(env(), cookieStore(), () => client)
      await assert.rejects(() => auth.signOut(), /network blip/)
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
