/**
 * The token format, replayed from the vectors the Rust crate generated.
 *
 * A device verifies these offline with `token.rs`; a Worker verifies the same
 * bytes with `token.ts`. A divergence between the two does not look like a
 * bug — it looks like an outage on one side or a bypass on the other — so the
 * format is pinned by vectors rather than by two people reading the same spec.
 */

import { describe, it, before } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

import {
  verifyToken,
  verifyTokenAs,
  decodeClaims,
  TOKEN_LEN,
  CLAIMS_LEN,
} from '../dist/index.js'

const here = dirname(fileURLToPath(import.meta.url))
const vectors = JSON.parse(
  readFileSync(join(here, '../../../docs/identity/token-vectors.json'), 'utf8'),
)

const fromHex = (s) => Uint8Array.from(s.match(/../g).map((b) => parseInt(b, 16)))

// `JSON.stringify` throws on a BigInt, and the nonce is one. An assertion
// message that crashes hides the assertion it was explaining.
const show = (v) => JSON.stringify(v, (_, x) => (typeof x === 'bigint' ? x.toString() : x))

describe('token format conformance', () => {
  let key

  before(async () => {
    key = await crypto.subtle.importKey(
      'raw',
      fromHex(vectors.public_key),
      { name: 'Ed25519' },
      false,
      ['verify'],
    )
  })

  it('the vectors describe the layout this implementation assumes', () => {
    // The constants are duplicated between the two languages by necessity.
    // This is what stops the duplication from becoming a disagreement.
    assert.equal(vectors.format.token_len, TOKEN_LEN)
    assert.equal(vectors.format.claims_len, CLAIMS_LEN)
    assert.equal(vectors.format.domain, 'fiducial-identity-token-v1')
  })

  for (const c of vectors.cases) {
    describe(`${c.name} — ${c.why}`, () => {
      it('the claims decode to what Rust signed', () => {
        const claims = decodeClaims(fromHex(c.token).subarray(0, CLAIMS_LEN))
        assert.ok(claims.ok, `decode failed: ${show(claims)}`)
        assert.deepEqual(claims.claims.principal, c.claims.principal)
        assert.equal(claims.claims.issuedAt, c.claims.issued_at)
        assert.equal(claims.claims.expiresAt, c.claims.expires_at)
        assert.equal(claims.claims.nonce, BigInt(c.claims.nonce))
      })

      for (const check of c.checks) {
        it(`at now=${check.now} → ${check.expect}`, async () => {
          const result = await verifyToken(key, fromHex(c.token), check.now ?? null)
          if (check.expect === 'ok') {
            assert.ok(result.ok, `expected ok, got ${show(result)}`)
          } else {
            assert.equal(result.ok, false)
            assert.equal(result.error, check.expect)
          }
        })
      }
    })
  }

  describe('rejections', () => {
    const sample = vectors.cases[0]
    const now = sample.claims.issued_at

    it('any tampered byte fails the signature', async () => {
      // Every signed byte, one at a time — a verifier covering only part of
      // the claims passes most tests and fails this one.
      for (let i = 0; i < CLAIMS_LEN; i++) {
        const bad = fromHex(sample.token)
        bad[i] ^= 0x01
        const result = await verifyToken(key, bad, now)
        assert.equal(result.ok, false, `byte ${i} is not covered`)
        assert.equal(result.error, 'bad_signature')
      }
    })

    it('a tampered signature fails', async () => {
      const bad = fromHex(sample.token)
      bad[TOKEN_LEN - 1] ^= 0x01
      const result = await verifyToken(key, bad, now)
      assert.equal(result.ok, false)
      assert.equal(result.error, 'bad_signature')
    })

    it('a truncated token is rejected before anything is parsed', async () => {
      const result = await verifyToken(key, fromHex(sample.token).subarray(0, TOKEN_LEN - 1), now)
      assert.equal(result.ok, false)
      assert.equal(result.error, 'malformed_length')
    })

    it('a token for another principal is refused by verifyTokenAs', async () => {
      // Without this the caller accepts any authentic token as any principal:
      // the signature is real, the binding to who is asking is missing.
      const someoneElse = { kind: 'user', id: 'f'.repeat(32) }
      const result = await verifyTokenAs(key, fromHex(sample.token), someoneElse, now)
      assert.equal(result.ok, false)
      assert.equal(result.error, 'wrong_principal')

      const right = await verifyTokenAs(key, fromHex(sample.token), sample.claims.principal, now)
      assert.ok(right.ok, show(right))
    })

    it('a token signed by another key is refused', async () => {
      const other = await crypto.subtle.generateKey({ name: 'Ed25519' }, true, ['sign', 'verify'])
      const result = await verifyToken(other.publicKey, fromHex(sample.token), now)
      assert.equal(result.ok, false)
      assert.equal(result.error, 'bad_signature')
    })

    it('a signature without the domain separator is refused', async () => {
      // A signature by the same key over the bare claims — what another
      // subsystem signing the same bytes would produce — must not verify.
      const pair = await crypto.subtle.generateKey({ name: 'Ed25519' }, true, ['sign', 'verify'])
      const claims = fromHex(sample.token).subarray(0, CLAIMS_LEN)
      const raw = new Uint8Array(await crypto.subtle.sign('Ed25519', pair.privateKey, claims))

      const forged = new Uint8Array(TOKEN_LEN)
      forged.set(claims, 0)
      forged.set(raw, CLAIMS_LEN)

      const result = await verifyToken(pair.publicKey, forged, now)
      assert.equal(result.ok, false)
      assert.equal(result.error, 'bad_signature')
    })
  })
})
