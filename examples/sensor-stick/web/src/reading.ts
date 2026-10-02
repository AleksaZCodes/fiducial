// A reading off the wire: the payload of one fiducial-protocol frame, laid
// out as the firmware writes it (firmware/shared/src/lib.rs → `reading`):
// [1, °C × 100 as i16 LE, %RH × 100 as u16 LE].

export const KIND = 1

export interface Reading {
  celsius: number
  percentRh: number
}

/** A payload as a reading, or null when it is some other kind of message. */
export function decode(payload: Uint8Array): Reading | null {
  if (payload.length !== 5 || payload[0] !== KIND) return null
  const view = new DataView(payload.buffer, payload.byteOffset, payload.byteLength)
  return {
    celsius: view.getInt16(1, true) / 100,
    percentRh: view.getUint16(3, true) / 100,
  }
}
