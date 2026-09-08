/**
 * TypeScript port of fiducial-protocol — byte-exact with the Rust crate.
 *
 * Frame format: [ MAGIC(1) | LEN_LO(1) | LEN_HI(1) | PAYLOAD(N) | CRC8(1) ]
 *
 * - MAGIC = 0xFD
 * - LEN = payload byte count, 2-byte little-endian
 * - CRC8 = XOR fold of all payload bytes
 */

export const MAGIC = 0xfd as const
export const MAX_PAYLOAD = 0xffff

export class EncodeError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'EncodeError'
  }
}

export function crc8(data: Uint8Array): number {
  let acc = 0
  for (const b of data) acc ^= b
  return acc
}

export function encodedLen(payloadLen: number): number {
  return payloadLen + 4
}

export function encode(payload: Uint8Array): Uint8Array {
  if (payload.length > MAX_PAYLOAD) throw new EncodeError('payload too large')
  const out = new Uint8Array(encodedLen(payload.length))
  out[0] = MAGIC
  out[1] = payload.length & 0xff
  out[2] = (payload.length >> 8) & 0xff
  out.set(payload, 3)
  out[3 + payload.length] = crc8(payload)
  return out
}

type DecodeState =
  | { tag: 'magic' }
  | { tag: 'len_lo' }
  | { tag: 'len_hi'; lenLo: number }
  | { tag: 'payload' }
  | { tag: 'crc' }

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
          this.state = { tag: 'crc' }
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
        if (this.pos >= this.expected) this.state = { tag: 'crc' }
        return null

      case 'crc': {
        const payload = this.buf.slice(0, this.expected)
        const computed = crc8(payload)
        this.state = { tag: 'magic' }
        return computed === byte ? payload : null
      }
    }
  }
}
