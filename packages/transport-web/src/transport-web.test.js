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
  crc8,
  encode,
  encodedLen,
  EncodeError,
  FrameDecoder,
  AsyncQueue,
} from '../dist/index.js'

// ── crc8 ──────────────────────────────────────────────────────────────────────

describe('crc8', () => {
  it('empty input returns 0', () => {
    assert.equal(crc8(new Uint8Array([])), 0)
  })
  it('single byte is returned as-is', () => {
    assert.equal(crc8(new Uint8Array([0x42])), 0x42)
  })
  it('two identical bytes XOR to 0', () => {
    assert.equal(crc8(new Uint8Array([0xAB, 0xAB])), 0)
  })
  it('"hi" = h ^ i', () => {
    const h = 'h'.charCodeAt(0)
    const i = 'i'.charCodeAt(0)
    assert.equal(crc8(new Uint8Array([h, i])), h ^ i)
  })
})

// ── encodedLen ────────────────────────────────────────────────────────────────

describe('encodedLen', () => {
  it('empty payload is 4 bytes', () => assert.equal(encodedLen(0), 4))
  it('10-byte payload is 14 bytes', () => assert.equal(encodedLen(10), 14))
  it('100-byte payload is 104 bytes', () => assert.equal(encodedLen(100), 104))
})

// ── encode ────────────────────────────────────────────────────────────────────

describe('encode', () => {
  it('starts with MAGIC byte', () => {
    assert.equal(encode(new Uint8Array([])).at(0), MAGIC)
  })

  it('empty payload: 4 bytes, crc=0', () => {
    const frame = encode(new Uint8Array([]))
    assert.equal(frame.length, 4)
    assert.equal(frame[0], MAGIC)
    assert.equal(frame[1], 0)
    assert.equal(frame[2], 0)
    assert.equal(frame[3], 0)
  })

  it('"hi" payload matches known layout', () => {
    const payload = new Uint8Array(['h', 'i'].map(c => c.charCodeAt(0)))
    const frame = encode(payload)
    assert.equal(frame[0], MAGIC)
    assert.equal(frame[1], 2)
    assert.equal(frame[2], 0)
    assert.equal(frame[3], 'h'.charCodeAt(0))
    assert.equal(frame[4], 'i'.charCodeAt(0))
    assert.equal(frame[5], 'h'.charCodeAt(0) ^ 'i'.charCodeAt(0))
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
