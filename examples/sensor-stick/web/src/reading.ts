// A reading off the wire. The payload's layout is not written here: it is
// declared once in protocol.toml and derived into protocol/messages.ts, the
// same table the firmware's encoder (protocol/messages.rs) comes from.

import { decodeReading } from '../../protocol/messages.ts'

export interface Reading {
  celsius: number
  percentRh: number
}

/** A payload as a reading, or null when it is some other kind of message. */
export function decode(payload: Uint8Array): Reading | null {
  const r = decodeReading(payload)
  if (r === null) return null
  return { celsius: r.centiCelsius / 100, percentRh: r.centiPercentRh / 100 }
}
