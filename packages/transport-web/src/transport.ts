/**
 * Common Transport interface — implemented by WebSerialTransport, WebUsbTransport,
 * and BleTransport. All three speak the same Fiducial frame codec.
 */
export interface Transport extends AsyncIterable<Uint8Array> {
  /** Encode `payload` as a Fiducial frame and write it to the device. */
  send(payload: Uint8Array): Promise<void>
  /** Release all resources and close the underlying connection. */
  close(): Promise<void>
}

/**
 * Minimal async queue used by event-driven transports (BLE) to bridge
 * characteristicvaluechanged callbacks into the async iterator.
 */
export class AsyncQueue<T> {
  private readonly items: T[] = []
  private readonly waiters: Array<(item: T | null) => void> = []
  private _closed = false

  push(item: T): void {
    if (this._closed) return
    if (this.waiters.length > 0) {
      this.waiters.shift()!(item)
    } else {
      this.items.push(item)
    }
  }

  close(): void {
    this._closed = true
    for (const w of this.waiters.splice(0)) w(null)
  }

  async pop(): Promise<T | null> {
    if (this.items.length > 0) return this.items.shift()!
    if (this._closed) return null
    return new Promise(resolve => this.waiters.push(resolve))
  }
}
