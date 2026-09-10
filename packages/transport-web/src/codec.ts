/**
 * TypeScript port of fiducial-protocol — byte-exact with the Rust crate.
 *
 * Frame format: [ MAGIC(1) | LEN_LO(1) | LEN_HI(1) | PAYLOAD(N) | CRC32(4) ]
 *
 * - MAGIC = 0xFD
 * - LEN = payload byte count, 2-byte little-endian
 * - CRC32 = CRC-32/ISO-HDLC over the payload, 4-byte little-endian
 *
 * Wire version 2. Version 1 used an 8-bit XOR fold, which is a parity byte
 * rather than a polynomial CRC and is not adequate over a 64 KB payload.
 * CRC-32/ISO-HDLC is the zlib/gzip/PNG/Ethernet variant, so a frame produced
 * here verifies against any conforming implementation on any platform.
 */

export const MAGIC = 0xfd as const
export const MAX_PAYLOAD = 0xffff

export class EncodeError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'EncodeError'
  }
}

/** Reflected polynomial for CRC-32/ISO-HDLC (0x04C11DB7 reflected). */
const CRC32_POLY = 0xedb88320

/**
 * CRC-32/ISO-HDLC — byte-exact with `fiducial_protocol::crc32` in Rust.
 *
 * Check value: `crc32(new TextEncoder().encode('123456789')) === 0xCBF43926`.
 *
 * All shifts use `>>>` and the result is coerced with `>>> 0` so the value
 * stays an unsigned 32-bit integer; JavaScript bitwise operators are otherwise
 * signed and would produce negative numbers here.
 */
export function crc32(data: Uint8Array): number {
  let crc = 0xffffffff
  for (const b of data) {
    crc ^= b
    for (let bit = 0; bit < 8; bit++) {
      crc = (crc & 1) !== 0 ? (crc >>> 1) ^ CRC32_POLY : crc >>> 1
    }
  }
  return (crc ^ 0xffffffff) >>> 0
}

export function encodedLen(payloadLen: number): number {
  return payloadLen + 7
}

export function encode(payload: Uint8Array): Uint8Array {
  if (payload.length > MAX_PAYLOAD) throw new EncodeError('payload too large')
  const out = new Uint8Array(encodedLen(payload.length))
  out[0] = MAGIC
  out[1] = payload.length & 0xff
  out[2] = (payload.length >> 8) & 0xff
  out.set(payload, 3)
  const crc = crc32(payload)
  const base = 3 + payload.length
  out[base] = crc & 0xff
  out[base + 1] = (crc >>> 8) & 0xff
  out[base + 2] = (crc >>> 16) & 0xff
  out[base + 3] = (crc >>> 24) & 0xff
  return out
}

type DecodeState =
  | { tag: 'magic' }
  | { tag: 'len_lo' }
  | { tag: 'len_hi'; lenLo: number }
  | { tag: 'payload' }
  | { tag: 'crc'; got: number; acc: number }

/**
 * Byte-by-byte frame decoder.
 *
 * Feed one byte at a time via `feed()`; it returns a Uint8Array of the decoded
 * payload when a complete, CRC-valid frame has arrived, or null otherwise.
 *
 * Mirrors the Rust `FrameDecoder<N>` state machine exactly — same resync and
 * oversized-frame rejection behaviour.
 */
export class FrameDecoder {
  private buf: Uint8Array
  private pos = 0
  private expected = 0
  private state: DecodeState = { tag: 'magic' }

  constructor(maxPayload = 4096) {
    this.buf = new Uint8Array(maxPayload)
  }

  reset(): void {
    this.pos = 0
    this.expected = 0
    this.state = { tag: 'magic' }
  }

  feed(byte: number): Uint8Array | null {
    const s = this.state
    switch (s.tag) {
      case 'magic':
        if (byte === MAGIC) this.state = { tag: 'len_lo' }
        return null

      case 'len_lo':
        this.state = { tag: 'len_hi', lenLo: byte }
        return null

      case 'len_hi': {
        const len = s.lenLo | (byte << 8)
        this.pos = 0
        this.expected = len
        if (len === 0) {
          this.state = { tag: 'crc', got: 0, acc: 0 }
        } else if (len > this.buf.length) {
          this.state = { tag: 'magic' }
        } else {
          this.state = { tag: 'payload' }
        }
        return null
      }

      case 'payload':
        if (this.pos < this.buf.length) this.buf[this.pos] = byte
        this.pos++
        if (this.pos >= this.expected) this.state = { tag: 'crc', got: 0, acc: 0 }
        return null

      case 'crc': {
        // Little-endian: first byte received is the least significant.
        const acc = (s.acc | (byte << (8 * s.got))) >>> 0
        const got = s.got + 1
        if (got < 4) {
          this.state = { tag: 'crc', got, acc }
          return null
        }
        const payload = this.buf.slice(0, this.expected)
        const computed = crc32(payload)
        this.state = { tag: 'magic' }
        return computed === acc ? payload : null
      }
    }
  }
}
