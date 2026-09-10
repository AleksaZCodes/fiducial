/**
 * Tests for @fiducial/transport-web.
 * Imports from ../dist — built by tsc before this test runs (turbo: test dependsOn build).
 *
 * Codec tests are pure JS and run in Node.
 * Transport tests use lightweight mocks — no browser required.
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import {
  MAGIC,
  MAX_PAYLOAD,
  crc32,
  encode,
  encodedLen,
  EncodeError,
  FrameDecoder,
  AsyncQueue,
} from '../dist/index.js'

// ── crc32 ─────────────────────────────────────────────────────────────────────

describe('crc32', () => {
  // The cross-implementation anchor: this is the CRC-32/ISO-HDLC check value,
  // asserted identically in the Rust crate. If these two ever disagree, the
  // TypeScript port has drifted from the wire format.
  it('standard check value for "123456789"', () => {
    const data = new TextEncoder().encode('123456789')
    assert.equal(crc32(data), 0xCBF43926)
  })
  it('empty input returns 0', () => {
    assert.equal(crc32(new Uint8Array([])), 0)
  })
  it('detects byte reordering', () => {
    const ab = new TextEncoder().encode('ab')
    const ba = new TextEncoder().encode('ba')
    assert.notEqual(crc32(ab), crc32(ba))
  })
  it('always returns an unsigned 32-bit value', () => {
    const data = new Uint8Array([0xFF, 0xFF, 0xFF, 0xFF])
    assert.ok(crc32(data) >= 0)
  })
})

// ── encodedLen ────────────────────────────────────────────────────────────────

describe('encodedLen', () => {
  it('empty payload is 7 bytes', () => assert.equal(encodedLen(0), 7))
  it('10-byte payload is 17 bytes', () => assert.equal(encodedLen(10), 17))
  it('100-byte payload is 107 bytes', () => assert.equal(encodedLen(100), 107))
})

// ── encode ────────────────────────────────────────────────────────────────────

describe('encode', () => {
  it('starts with MAGIC byte', () => {
    assert.equal(encode(new Uint8Array([])).at(0), MAGIC)
  })

  it('empty payload: 7 bytes, crc=0', () => {
    const frame = encode(new Uint8Array([]))
    assert.equal(frame.length, 7)
    assert.equal(frame[0], MAGIC)
    assert.equal(frame[1], 0)
    assert.equal(frame[2], 0)
    assert.deepEqual(Array.from(frame.slice(3, 7)), [0, 0, 0, 0])
  })

  it('"hi" payload matches known layout', () => {
    const payload = new Uint8Array(['h', 'i'].map(c => c.charCodeAt(0)))
    const frame = encode(payload)
    assert.equal(frame[0], MAGIC)
    assert.equal(frame[1], 2)
    assert.equal(frame[2], 0)
    assert.equal(frame[3], 'h'.charCodeAt(0))
    assert.equal(frame[4], 'i'.charCodeAt(0))
    const crc = crc32(payload)
    assert.deepEqual(Array.from(frame.slice(5, 9)), [
      crc & 0xff,
      (crc >>> 8) & 0xff,
      (crc >>> 16) & 0xff,
      (crc >>> 24) & 0xff,
    ])
  })

  it('length field is little-endian', () => {
    const payload = new Uint8Array(300).fill(0xAA)
    const frame = encode(payload)
    const len = frame[1] | (frame[2] << 8)
    assert.equal(len, 300)
  })

  it('throws EncodeError when payload exceeds MAX_PAYLOAD', () => {
    const huge = new Uint8Array(MAX_PAYLOAD + 1)
    assert.throws(() => encode(huge), EncodeError)
  })
})

// ── FrameDecoder ──────────────────────────────────────────────────────────────

function feedAll(decoder, bytes) {
  for (const b of bytes) {
    const result = decoder.feed(b)
    if (result !== null) return result
  }
  return null
}

describe('FrameDecoder', () => {
  it('roundtrip: empty payload', () => {
    const dec = new FrameDecoder()
    const frame = encode(new Uint8Array([]))
    const result = feedAll(dec, frame)
    assert.deepEqual(result, new Uint8Array([]))
  })

  it('roundtrip: "hello"', () => {
    const dec = new FrameDecoder()
    const payload = new Uint8Array([...Array.from('hello')].map(c => c.charCodeAt(0)))
    const frame = encode(payload)
    const result = feedAll(dec, frame)
    assert.deepEqual(result, payload)
  })

  it('drops frame on CRC failure and returns null', () => {
    const dec = new FrameDecoder()
    const frame = encode(new Uint8Array([0x01, 0x02, 0x03]))
    frame[frame.length - 1] ^= 0xFF
    assert.equal(feedAll(dec, frame), null)
  })

  it('resyncs after leading noise bytes', () => {
    const dec = new FrameDecoder()
    const noise = new Uint8Array([0x00, 0x12, 0x34])
    const payload = new Uint8Array([...'sync'].map(c => c.charCodeAt(0)))
    const validFrame = encode(payload)
    const input = new Uint8Array([...noise, ...validFrame])
    const result = feedAll(dec, input)
    assert.deepEqual(result, payload)
  })

  it('drops oversized frame (claimed len > buffer) and resyncs', () => {
    const dec = new FrameDecoder(64)
    const claimedLen = 300
    const oversized = new Uint8Array([MAGIC, claimedLen & 0xff, (claimedLen >> 8) & 0xff, ...new Uint8Array(300)])
    const validPayload = new Uint8Array([0xAA])
    const validFrame = encode(validPayload)
    const input = new Uint8Array([...oversized, ...validFrame])
    const result = feedAll(dec, input)
    assert.deepEqual(result, validPayload)
  })

  it('reset() discards partial state', () => {
    const dec = new FrameDecoder()
    dec.feed(MAGIC)
    dec.feed(0x05)
    dec.reset()
    const payload = new Uint8Array([0x01])
    const frame = encode(payload)
    const result = feedAll(dec, frame)
    assert.deepEqual(result, payload)
  })

  it('two consecutive frames decoded correctly', () => {
    const dec = new FrameDecoder()
    const a = new Uint8Array([0x01])
    const b = new Uint8Array([0x02, 0x03])
    const input = new Uint8Array([...encode(a), ...encode(b)])
    const results = []
    for (const byte of input) {
      const r = dec.feed(byte)
      if (r !== null) results.push(r)
    }
    assert.equal(results.length, 2)
    assert.deepEqual(results[0], a)
    assert.deepEqual(results[1], b)
  })
})

// ── AsyncQueue ────────────────────────────────────────────────────────────────

describe('AsyncQueue', () => {
  it('push then pop returns the item', async () => {
    const q = new AsyncQueue()
    q.push(42)
    assert.equal(await q.pop(), 42)
  })

  it('pop before push: resolves when push arrives', async () => {
    const q = new AsyncQueue()
    const promise = q.pop()
    q.push('hello')
    assert.equal(await promise, 'hello')
  })

  it('close() causes pop to return null', async () => {
    const q = new AsyncQueue()
    const promise = q.pop()
    q.close()
    assert.equal(await promise, null)
  })

  it('push after close() is a no-op', async () => {
    const q = new AsyncQueue()
    q.close()
    q.push(99)
    assert.equal(await q.pop(), null)
  })

  it('preserves FIFO order', async () => {
    const q = new AsyncQueue()
    q.push(1)
    q.push(2)
    q.push(3)
    assert.equal(await q.pop(), 1)
    assert.equal(await q.pop(), 2)
    assert.equal(await q.pop(), 3)
  })
})

// ── Conformance vectors ───────────────────────────────────────────────────────
//
// The cross-implementation anchor. These vectors are generated from the Rust
// crate and committed at docs/protocol/vectors.json; this suite and the Rust
// suite both assert against that one file.
//
// Two hand-written implementations of one spec drift, and the drift is silent
// until a device stops talking to a browser. A third artifact both answer to is
// what makes "byte-exact" a checked property instead of a claim in a comment.

import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'

const here = dirname(fileURLToPath(import.meta.url))
const vectorsPath = join(here, '..', '..', '..', 'docs', 'protocol', 'vectors.json')
const vectors = JSON.parse(readFileSync(vectorsPath, 'utf8'))

const fromHex = (h) =>
  new Uint8Array((h.match(/../g) ?? []).map((b) => parseInt(b, 16)))
const toHex = (u8) =>
  Array.from(u8, (b) => b.toString(16).padStart(2, '0')).join('')

describe('conformance vectors (docs/protocol/vectors.json)', () => {
  it('agrees with Rust on the wire version', () => {
    assert.equal(vectors.wire_version, 2)
  })

  it('agrees on MAGIC and frame overhead', () => {
    assert.equal(MAGIC, parseInt(vectors.magic, 16))
    assert.equal(encodedLen(0), vectors.frame_overhead)
  })

  it('agrees on MAX_PAYLOAD', () => {
    assert.equal(MAX_PAYLOAD, vectors.max_payload)
  })

  it('has vectors to check', () => {
    assert.ok(vectors.vectors.length > 0)
  })

  for (const v of vectors.vectors) {
    it(`crc32 matches Rust for "${v.name}"`, () => {
      const got = crc32(fromHex(v.payload))
      assert.equal(
        '0x' + got.toString(16).toUpperCase().padStart(8, '0'),
        v.crc32,
      )
    })

    it(`encodes byte-for-byte like Rust for "${v.name}"`, () => {
      assert.equal(toHex(encode(fromHex(v.payload))), v.frame)
    })

    it(`decodes the Rust-generated frame for "${v.name}"`, () => {
      const dec = new FrameDecoder(1024)
      const frame = fromHex(v.frame)
      let got = null
      for (const b of frame) {
        const out = dec.feed(b)
        if (out !== null) got = out
      }
      assert.equal(toHex(got ?? new Uint8Array()), v.payload)
    })
  }
})
