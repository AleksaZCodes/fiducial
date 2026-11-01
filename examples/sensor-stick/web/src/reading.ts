// A reading off the wire, and how the page shows it. The payload's layout,
// scale and units are not written here: they are declared once in
// protocol.toml and derived into protocol/messages.ts, the same table the
// firmware's encoder (protocol/messages.rs) comes from.

import { READING_UNITS, decodeReading, readingBand, readingValues } from '../../protocol/messages.ts'

export interface Reading {
  celsius: number
  percentRh: number
}

/** A payload as a reading, or null when it is some other kind of message. */
export function decode(payload: Uint8Array): Reading | null {
  const r = decodeReading(payload)
  if (r === null) return null
  const v = readingValues(r)
  return { celsius: v.centiCelsius, percentRh: v.centiPercentRh }
}

/** What the page shows for a reading — and, inside the declared band, that it is comfortable (the LED is green then too). */
export function show(r: Reading): string {
  const comfortable = readingBand('comfortable', r.percentRh) === 'inside' ? ' · comfortable' : ''
  return `${r.celsius.toFixed(1)} ${READING_UNITS.centiCelsius} · ${r.percentRh.toFixed(0)} ${READING_UNITS.centiPercentRh}${comfortable}`
}
