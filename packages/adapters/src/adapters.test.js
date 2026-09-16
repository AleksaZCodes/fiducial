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
  EmailError,
  ResendEmail,
  NoneDiagnostics,
  NoneBotProtection,
  Turnstile,
  BotProtectionError,
  NoneQueue,
  CloudflareQueue,
  QueueError,
  NoneNewsletter,
  NewsletterError,
  ResendNewsletter,
  NoneAuth,
  AuthError,
  CookieKeyValueStore,
  CookieSessionContext,
  BearerSessionContext,
  MemoryKeyValueStore,
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

  describe('BearerSessionContext', () => {
    it('fromAuthorizationHeader extracts the bearer token', () => {
      const ctx = BearerSessionContext.fromAuthorizationHeader('Bearer abc123')
      assert.equal(ctx.bearerToken, 'abc123')
    })

    it('is case-insensitive about the scheme', () => {
      assert.equal(
        BearerSessionContext.fromAuthorizationHeader('bearer abc123').bearerToken,
        'abc123',
      )
    })

    it('with no header carries no token', () => {
      assert.equal(BearerSessionContext.fromAuthorizationHeader(null).bearerToken, null)
    })

    it('ignores a non-bearer scheme', () => {
      assert.equal(
        BearerSessionContext.fromAuthorizationHeader('Basic dXNlcjpwdw==').bearerToken,
        null,
      )
    })

    it('carries an in-memory storage for the SDK scratch use', () => {
      const ctx = new BearerSessionContext('tok')
      assert.ok(ctx.storage instanceof MemoryKeyValueStore)
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
          // Verification: the real SDK checks the signature here (locally via
          // JWKS, or by calling getUser). The fake says "valid, subject is
          // user-1" unless a test overrides it to reject.
          async getClaims(jwt) {
            return {
              data: { claims: { sub: 'user-1', email: 'a@example.com', exp: 9999999999 } },
              error: null,
            }
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

    function cookieCtx() {
      const jar = new Map()
      return new CookieSessionContext({
        get: (n) => jar.get(n),
        set: (n, v) => jar.set(n, v),
        delete: (n) => jar.delete(n),
      })
    }

    it('throws when env is missing SUPABASE_URL/SUPABASE_ANON_KEY', () => {
      assert.throws(() => new SupabaseAuth({}, cookieCtx(), fakeSupabaseClient), AuthError)
    })

    it('signUp maps the returned session', async () => {
      const auth = new SupabaseAuth(env(), cookieCtx(), () => fakeSupabaseClient())
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
      const auth = new SupabaseAuth(env(), cookieCtx(), () => client)
      await assert.rejects(() => auth.signUp('a@example.com', 'pw'), AuthError)
    })

    it('signIn maps the returned session', async () => {
      const auth = new SupabaseAuth(env(), cookieCtx(), () => fakeSupabaseClient())
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
      const auth = new SupabaseAuth(env(), cookieCtx(), () => client)
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
      const auth = new SupabaseAuth(env(), cookieCtx(), () => client)
      await assert.rejects(() => auth.signIn('a@example.com', 'pw'), /rate limited/)
    })

    it('signInWithOAuth returns the redirect URL under a cookie store', async () => {
      const auth = new SupabaseAuth(env(), cookieCtx(), () => fakeSupabaseClient())
      const { url } = await auth.signInWithOAuth('google', 'https://app.example/callback')
      assert.equal(url, 'https://provider.example/authorize')
    })

    it('signInWithOAuth throws under a bearer store', async () => {
      const auth = new SupabaseAuth(
        env(),
        new BearerSessionContext(),
        () => fakeSupabaseClient(),
      )
      await assert.rejects(
        () => auth.signInWithOAuth('google', 'https://app.example/callback'),
        AuthError,
      )
    })

    it('exchangeCodeForSession maps the session under a cookie store', async () => {
      const auth = new SupabaseAuth(env(), cookieCtx(), () => fakeSupabaseClient())
      const session = await auth.exchangeCodeForSession('code-123')
      assert.equal(session.accessToken, 'access-tok')
    })

    it('exchangeCodeForSession throws under a bearer store', async () => {
      const auth = new SupabaseAuth(
        env(),
        new BearerSessionContext(),
        () => fakeSupabaseClient(),
      )
      await assert.rejects(() => auth.exchangeCodeForSession('code-123'), AuthError)
    })

    it('signOut calls through and resolves', async () => {
      const auth = new SupabaseAuth(env(), cookieCtx(), () => fakeSupabaseClient())
      await auth.signOut()
    })

    it('getSession returns null when the client reports none', async () => {
      const client = fakeSupabaseClient({
        async getSession() {
          return { data: { session: null }, error: null }
        },
      })
      const auth = new SupabaseAuth(env(), cookieCtx(), () => client)
      assert.equal(await auth.getSession(), null)
    })

    it('getSession maps a present session', async () => {
      const auth = new SupabaseAuth(env(), cookieCtx(), () => fakeSupabaseClient())
      const session = await auth.getSession()
      assert.equal(session.user.id, 'user-1')
    })

    // ── Verification ─────────────────────────────────────────────────────
    // The SDK's own getSession() reads the session out of storage and checks
    // only expires_at. On a server, storage is a *client-supplied cookie* —
    // so without these, a forged cookie is a valid session with any user id
    // the caller likes.

    it('getSession refuses a session whose token does not verify', async () => {
      const client = fakeSupabaseClient({
        // A forged cookie: well-formed session, signature does not check out.
        async getClaims() {
          return { data: null, error: { message: 'invalid JWT signature' } }
        },
      })
      const auth = new SupabaseAuth(env(), cookieCtx(), () => client)
      assert.equal(
        await auth.getSession(),
        null,
        'an unverified token must never become a session',
      )
    })

    it('getSession refuses a cookie whose stored user is not the token subject', async () => {
      const client = fakeSupabaseClient({
        // Valid token for user-1, but the cookie claims to be somebody else.
        async getClaims() {
          return { data: { claims: { sub: 'user-1', exp: 9999999999 } }, error: null }
        },
        async getSession() {
          const s = fakeSession()
          s.user = { ...fakeUser(), id: 'somebody-else' }
          return { data: { session: s }, error: null }
        },
      })
      const auth = new SupabaseAuth(env(), cookieCtx(), () => client)
      assert.equal(await auth.getSession(), null)
    })

    it('bearer delivery verifies the supplied token and builds a session from its claims', async () => {
      const client = fakeSupabaseClient({
        // Nothing is in storage under bearer delivery — if the adapter asked
        // the SDK for a stored session it would get null, which is exactly
        // the bug this path used to have.
        async getSession() {
          return { data: { session: null }, error: null }
        },
      })
      const auth = new SupabaseAuth(
        env(),
        BearerSessionContext.fromAuthorizationHeader('Bearer forged-or-not'),
        () => client,
      )
      const session = await auth.getSession()
      assert.ok(session, 'a verified bearer token yields a session')
      assert.equal(session.user.id, 'user-1')
      assert.equal(session.accessToken, 'forged-or-not')
      assert.equal(
        session.refreshToken,
        null,
        'the server never sees a bearer client’s refresh token',
      )
      assert.equal(
        session.user.emailVerified,
        null,
        'a JWT does not carry confirmation state; null says so rather than guessing',
      )
    })

    it('bearer delivery refuses a token that does not verify', async () => {
      const client = fakeSupabaseClient({
        async getClaims() {
          return { data: null, error: { message: 'invalid JWT signature' } }
        },
      })
      const auth = new SupabaseAuth(
        env(),
        BearerSessionContext.fromAuthorizationHeader('Bearer nope'),
        () => client,
      )
      assert.equal(await auth.getSession(), null)
    })

    it('bearer delivery with no token is signed out', async () => {
      const auth = new SupabaseAuth(
        env(),
        BearerSessionContext.fromAuthorizationHeader(null),
        () => fakeSupabaseClient({
          async getSession() {
            return { data: { session: null }, error: null }
          },
        }),
      )
      assert.equal(await auth.getSession(), null)
    })

    it('resetPasswordForEmail calls through', async () => {
      const auth = new SupabaseAuth(env(), cookieCtx(), () => fakeSupabaseClient())
      await auth.resetPasswordForEmail('a@example.com', 'https://app.example/reset')
    })

    it('updatePassword calls through', async () => {
      const auth = new SupabaseAuth(env(), cookieCtx(), () => fakeSupabaseClient())
      await auth.updatePassword('new-password')
    })

    it('propagates a generic vendor error', async () => {
      const client = fakeSupabaseClient({
        async signOut() {
          return { error: { message: 'network blip' } }
        },
      })
      const auth = new SupabaseAuth(env(), cookieCtx(), () => client)
      await assert.rejects(() => auth.signOut(), /network blip/)
    })
  })

  // ── ResendEmail ──────────────────────────────────────────────────────────────

  describe('ResendEmail', () => {
    it('throws EmailError when no API key is present', () => {
      assert.throws(() => new ResendEmail({}), EmailError)
    })

    it('posts to Resend and returns the message id on success', async () => {
      let capturedUrl
      let capturedBody
      let capturedHeaders
      const fakeFetch = async (url, init) => {
        capturedUrl = url
        capturedHeaders = init.headers
        capturedBody = JSON.parse(init.body)
        return {
          ok: true,
          status: 200,
          async json() { return { id: 'msg-abc-123' } },
        }
      }
      const email = new ResendEmail({ RESEND_API_KEY: 're_key' }, fakeFetch)
      const id = await email.send({
        from: 'hi@example.com',
        to: ['user@example.com'],
        subject: 'Hello',
        html: '<p>Hi</p>',
      })
      assert.equal(capturedUrl, 'https://api.resend.com/emails')
      assert.equal(capturedHeaders['Authorization'], 'Bearer re_key')
      assert.equal(capturedBody.from, 'hi@example.com')
      assert.deepEqual(capturedBody.to, ['user@example.com'])
      assert.equal(id, 'msg-abc-123')
    })

    it('throws EmailError on HTTP 429 (rate limited)', async () => {
      const fakeFetch = async () => ({ ok: false, status: 429 })
      const email = new ResendEmail({ RESEND_API_KEY: 're_key' }, fakeFetch)
      await assert.rejects(() => email.send({
        from: 'hi@example.com',
        to: ['u@example.com'],
        subject: 'Hi',
        html: '<p>Hi</p>',
      }), EmailError)
    })

    it('throws EmailError on a non-OK HTTP response', async () => {
      const fakeFetch = async () => ({ ok: false, status: 503 })
      const email = new ResendEmail({ RESEND_API_KEY: 're_key' }, fakeFetch)
      await assert.rejects(() => email.send({
        from: 'hi@example.com',
        to: ['u@example.com'],
        subject: 'Hi',
        html: '<p>Hi</p>',
      }), EmailError)
    })
  })

  // ── NoneNewsletter ────────────────────────────────────────────────────────────

  describe('NoneNewsletter', () => {
    it('subscribe returns a plausible Subscription with empty id', async () => {
      const nl = new NoneNewsletter()
      const sub = await nl.subscribe('a@example.com')
      assert.equal(sub.id, '')
      assert.equal(sub.email, 'a@example.com')
      assert.equal(sub.subscribed, true)
    })

    it('subscribe with attributes still returns a Subscription', async () => {
      const nl = new NoneNewsletter()
      const sub = await nl.subscribe('a@example.com', { firstName: 'Alice' })
      assert.equal(sub.subscribed, true)
    })

    it('unsubscribe succeeds silently', async () => {
      const nl = new NoneNewsletter()
      await nl.unsubscribe('a@example.com')
    })

    it('status returns null', async () => {
      const nl = new NoneNewsletter()
      assert.equal(await nl.status('a@example.com'), null)
    })
  })

  // ── ResendNewsletter ──────────────────────────────────────────────────────────

  describe('ResendNewsletter', () => {
    it('throws NewsletterError when API key is absent', () => {
      assert.throws(
        () => new ResendNewsletter({ RESEND_AUDIENCE_ID: 'aud-1' }),
        NewsletterError,
      )
    })

    it('throws NewsletterError when audience ID is absent', () => {
      assert.throws(
        () => new ResendNewsletter({ RESEND_API_KEY: 're_key' }),
        NewsletterError,
      )
    })

    it('subscribe (fresh contact) posts and returns a Subscription', async () => {
      let capturedUrl
      let capturedBody
      const fakeFetch = async (url, init) => {
        capturedUrl = url
        capturedBody = JSON.parse(init.body)
        return {
          ok: true,
          status: 200,
          async json() { return { object: 'contact', id: 'cid-1' } },
        }
      }
      const nl = new ResendNewsletter(
        { RESEND_API_KEY: 're_key', RESEND_AUDIENCE_ID: 'aud-1' },
        fakeFetch,
      )
      const sub = await nl.subscribe('a@example.com', { firstName: 'Alice' })
      assert.equal(capturedUrl, 'https://api.resend.com/audiences/aud-1/contacts')
      assert.equal(capturedBody.email, 'a@example.com')
      assert.equal(capturedBody.first_name, 'Alice')
      assert.equal(sub.id, 'cid-1')
      assert.equal(sub.email, 'a@example.com')
      assert.equal(sub.subscribed, true)
    })

    it('subscribe (already-exists 409) falls back to status and returns existing record', async () => {
      let callCount = 0
      const fakeFetch = async (url, init) => {
        callCount++
        if ((init?.method ?? 'GET') === 'POST') {
          return { ok: false, status: 409, async json() { return {} } }
        }
        // GET /audiences/{id}/contacts/{email}
        return {
          ok: true,
          status: 200,
          async json() {
            return { id: 'cid-existing', email: 'a@example.com', unsubscribed: false }
          },
        }
      }
      const nl = new ResendNewsletter(
        { RESEND_API_KEY: 're_key', RESEND_AUDIENCE_ID: 'aud-1' },
        fakeFetch,
      )
      const sub = await nl.subscribe('a@example.com')
      assert.equal(sub.id, 'cid-existing')
      assert.equal(sub.subscribed, true)
      assert.equal(callCount, 2)
    })

    it('unsubscribe PATCHes the contact with unsubscribed=true', async () => {
      let capturedUrl
      let capturedBody
      const fakeFetch = async (url, init) => {
        capturedUrl = url
        capturedBody = JSON.parse(init.body)
        return {
          ok: true,
          status: 200,
          async json() {
            return { id: 'cid-1', email: 'a@example.com', unsubscribed: true }
          },
        }
      }
      const nl = new ResendNewsletter(
        { RESEND_API_KEY: 're_key', RESEND_AUDIENCE_ID: 'aud-1' },
        fakeFetch,
      )
      await nl.unsubscribe('a@example.com')
      assert.ok(capturedUrl.includes('/contacts/a%40example.com'), capturedUrl)
      assert.equal(capturedBody.unsubscribed, true)
    })

    it('status returns a Subscription when the contact is found', async () => {
      const fakeFetch = async () => ({
        ok: true,
        status: 200,
        async json() {
          return { id: 'cid-1', email: 'a@example.com', unsubscribed: false }
        },
      })
      const nl = new ResendNewsletter(
        { RESEND_API_KEY: 're_key', RESEND_AUDIENCE_ID: 'aud-1' },
        fakeFetch,
      )
      const sub = await nl.status('a@example.com')
      assert.ok(sub !== null)
      assert.equal(sub.id, 'cid-1')
      assert.equal(sub.subscribed, true)
    })

    it('status returns null on 404', async () => {
      const fakeFetch = async () => ({ ok: false, status: 404 })
      const nl = new ResendNewsletter(
        { RESEND_API_KEY: 're_key', RESEND_AUDIENCE_ID: 'aud-1' },
        fakeFetch,
      )
      assert.equal(await nl.status('nobody@example.com'), null)
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
      assert.ok(adapters.newsletter instanceof NoneNewsletter)
    })
  })
})
