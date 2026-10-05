// Derived by fid-protocol from protocol.toml — do not edit.
// The firmware's protocol/messages.rs is derived from the same table.

/** One temperature and humidity reading, sent once a second. */
export const READING_KIND = 1
export const READING_LEN = 5

export interface Reading {
  /** Temperature, °C × 100 */
  centiCelsius: number
  /** Relative humidity, %RH × 100 */
  centiPercentRh: number
}

/** A payload as a `reading`, or null when it is another kind of message. */
export function decodeReading(p: Uint8Array): Reading | null {
  if (p.length !== 5 || p[0] !== 1) return null
  const v = new DataView(p.buffer, p.byteOffset, p.byteLength)
  return {
    centiCelsius: v.getInt16(1, true),
    centiPercentRh: v.getUint16(3, true),
  }
}

export function encodeReading(m: Reading): Uint8Array {
  const p = new Uint8Array(5)
  const v = new DataView(p.buffer)
  p[0] = 1
  v.setInt16(1, m.centiCelsius, true)
  v.setUint16(3, m.centiPercentRh, true)
  return p
}

/** Each field's unit, as declared. */
export const READING_UNITS = {
  centiCelsius: "°C",
  centiPercentRh: "%RH",
} as const

/** A `reading` in its units: each field times its declared scale. */
export function readingValues(m: Reading): Reading {
  return {
    centiCelsius: m.centiCelsius * 0.01,
    centiPercentRh: m.centiPercentRh * 0.01,
  }
}
