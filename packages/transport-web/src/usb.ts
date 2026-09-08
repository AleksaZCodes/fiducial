/**
 * WebUSB transport for Fiducial.
 *
 * Requires a browser that supports the WebUSB API (Chrome/Edge 61+).
 * The device must be running firmware with a vendor-specific USB interface
 * exposing bulk IN and OUT endpoints on interface 0.
 *
 * Usage:
 *   const transport = await WebUsbTransport.open({ filters: [{ vendorId: 0x1209 }] })
 *   await transport.send(new Uint8Array([0x01, 0x02]))
 *   for await (const frame of transport) { ... }
 *   await transport.close()
 */

import { encode, FrameDecoder } from './codec.js'
import type { Transport } from './transport.js'

export interface UsbOptions {
  filters?: USBDeviceFilter[]
  /** USB interface number to claim (default: 0). */
  interfaceNumber?: number
  /** Bulk OUT endpoint number (default: 1). */
  endpointOut?: number
  /** Bulk IN endpoint number (default: 1). */
  endpointIn?: number
  /** Max bytes per IN transfer (default: 64). */
  packetSize?: number
}

export class WebUsbTransport implements Transport {
  private readonly device: USBDevice
  private readonly decoder: FrameDecoder
  private readonly interfaceNumber: number
  private readonly endpointOut: number
  private readonly endpointIn: number
  private readonly packetSize: number
  private _closed = false

  private constructor(device: USBDevice, opts: Required<Omit<UsbOptions, 'filters'>>) {
    this.device = device
    this.decoder = new FrameDecoder()
    this.interfaceNumber = opts.interfaceNumber
    this.endpointOut = opts.endpointOut
    this.endpointIn = opts.endpointIn
    this.packetSize = opts.packetSize
  }

  static async open(options: UsbOptions = {}): Promise<WebUsbTransport> {
    const {
      filters = [],
      interfaceNumber = 0,
      endpointOut = 1,
      endpointIn = 1,
      packetSize = 64,
    } = options
    const device = await navigator.usb.requestDevice({ filters })
    await device.open()
    await device.selectConfiguration(1)
    await device.claimInterface(interfaceNumber)
    return new WebUsbTransport(device, { interfaceNumber, endpointOut, endpointIn, packetSize })
  }

  async send(payload: Uint8Array): Promise<void> {
    await this.device.transferOut(this.endpointOut, encode(payload))
  }

  async close(): Promise<void> {
    this._closed = true
    await this.device.releaseInterface(this.interfaceNumber)
    await this.device.close()
  }

  async *[Symbol.asyncIterator](): AsyncGenerator<Uint8Array> {
    while (!this._closed) {
      let result: USBInTransferResult
      try {
        result = await this.device.transferIn(this.endpointIn, this.packetSize)
      } catch {
        break
      }
      if (result.data) {
        for (let i = 0; i < result.data.byteLength; i++) {
          const frame = this.decoder.feed(result.data.getUint8(i))
          if (frame !== null) yield frame
        }
      }
    }
  }
}
