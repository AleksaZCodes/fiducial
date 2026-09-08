/**
 * Web Serial transport for Fiducial.
 *
 * Requires a browser that supports the Web Serial API (Chrome/Edge 89+).
 * The device must be running firmware that speaks the Fiducial frame protocol.
 *
 * Usage:
 *   const transport = await WebSerialTransport.open()
 *   await transport.send(new Uint8Array([0x01, 0x02]))
 *   for await (const frame of transport) { ... }
 *   await transport.close()
 */

import { encode, FrameDecoder } from './codec.js'
import type { Transport } from './transport.js'

export interface SerialOptions {
  baudRate?: number
  filters?: SerialPortFilter[]
  /**
   * Largest payload the decoder will accept, in bytes (default: 4096).
   *
   * Frames declaring a longer payload are silently dropped, so this must be at
   * least as large as the biggest frame the device sends. `encode()` permits up
   * to `MAX_PAYLOAD` (65535).
   */
  maxPayload?: number
}

export class WebSerialTransport implements Transport {
  private readonly port: SerialPort
  private readonly writer: WritableStreamDefaultWriter<Uint8Array>
  private readonly decoder: FrameDecoder
  private readonly baudRate: number

  private constructor(
    port: SerialPort,
    writer: WritableStreamDefaultWriter<Uint8Array>,
    baudRate: number,
    maxPayload: number,
  ) {
    this.port = port
    this.writer = writer
    this.decoder = new FrameDecoder(maxPayload)
    this.baudRate = baudRate
  }

  static async open(options: SerialOptions = {}): Promise<WebSerialTransport> {
    const { baudRate = 115200, filters, maxPayload = 4096 } = options
    const port = await navigator.serial.requestPort({ filters: filters ?? [] })
    await port.open({ baudRate })
    const writer = port.writable!.getWriter()
    return new WebSerialTransport(port, writer, baudRate, maxPayload)
  }

  async send(payload: Uint8Array): Promise<void> {
    await this.writer.write(encode(payload))
  }

  async close(): Promise<void> {
    await this.writer.close()
    await this.port.close()
  }

  async *[Symbol.asyncIterator](): AsyncGenerator<Uint8Array> {
    const reader = this.port.readable!.getReader()
    try {
      while (true) {
        const { value, done } = await reader.read()
        if (done) break
        for (const byte of value) {
          const frame = this.decoder.feed(byte)
          if (frame !== null) yield frame
        }
      }
    } finally {
      reader.releaseLock()
    }
  }
}
