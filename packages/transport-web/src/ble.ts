/**
 * Web Bluetooth (BLE) transport for Fiducial.
 *
 * Requires a browser that supports the Web Bluetooth API (Chrome/Edge 56+).
 * The device must advertise the Fiducial BLE service and expose two
 * characteristics: TX (notifications, device→browser) and RX (write,
 * browser→device).
 *
 * Fiducial BLE UUIDs:
 *   Service : 0000fd01-0000-1000-8000-00805f9b34fb
 *   TX char : 0000fd02-0000-1000-8000-00805f9b34fb  (notify)
 *   RX char : 0000fd03-0000-1000-8000-00805f9b34fb  (write)
 *
 * Usage:
 *   const transport = await BleTransport.open()
 *   await transport.send(new Uint8Array([0x01, 0x02]))
 *   for await (const frame of transport) { ... }
 *   await transport.close()
 */

import { encode, FrameDecoder } from './codec.js'
import { AsyncQueue } from './transport.js'
import type { Transport } from './transport.js'

/**
 * Default Fiducial GATT service UUID.
 *
 * PLACEHOLDER — `fd01`/`fd02`/`fd03` sit in the 16-bit SIG range and are not
 * allocated to Fiducial. Ship a product on BLE only after replacing these with
 * your own UUIDs via {@link BleOptions}; changing them after devices are in the
 * field breaks every existing pairing.
 */
export const FIDUCIAL_SERVICE_UUID = '0000fd01-0000-1000-8000-00805f9b34fb'
/** Default TX characteristic — device notifies browser (incoming frames). */
export const FIDUCIAL_TX_UUID = '0000fd02-0000-1000-8000-00805f9b34fb'
/** Default RX characteristic — browser writes to device. */
export const FIDUCIAL_RX_UUID = '0000fd03-0000-1000-8000-00805f9b34fb'

export interface BleOptions {
  /** Additional device filters (default: accept any device with `serviceUuid`). */
  filters?: BluetoothLEScanFilter[]
  /** GATT service UUID (default: {@link FIDUCIAL_SERVICE_UUID}). */
  serviceUuid?: string
  /** Notify characteristic UUID (default: {@link FIDUCIAL_TX_UUID}). */
  txUuid?: string
  /** Write characteristic UUID (default: {@link FIDUCIAL_RX_UUID}). */
  rxUuid?: string
}

export class BleTransport implements Transport {
  private readonly device: BluetoothDevice
  private readonly rxChar: BluetoothRemoteGATTCharacteristic
  private readonly decoder: FrameDecoder
  private readonly queue: AsyncQueue<Uint8Array>
  private readonly onNotify: (event: Event) => void

  private constructor(
    device: BluetoothDevice,
    txChar: BluetoothRemoteGATTCharacteristic,
    rxChar: BluetoothRemoteGATTCharacteristic,
  ) {
    this.device = device
    this.rxChar = rxChar
    this.decoder = new FrameDecoder()
    this.queue = new AsyncQueue()

    this.onNotify = (event: Event) => {
      const char = event.target as BluetoothRemoteGATTCharacteristic
      if (!char.value) return
      for (let i = 0; i < char.value.byteLength; i++) {
        const frame = this.decoder.feed(char.value.getUint8(i))
        if (frame !== null) this.queue.push(frame)
      }
    }

    txChar.addEventListener('characteristicvaluechanged', this.onNotify)
  }

  static async open(options: BleOptions = {}): Promise<BleTransport> {
    const serviceUuid = options.serviceUuid ?? FIDUCIAL_SERVICE_UUID
    const txUuid = options.txUuid ?? FIDUCIAL_TX_UUID
    const rxUuid = options.rxUuid ?? FIDUCIAL_RX_UUID

    const filters = options.filters ?? [{ services: [serviceUuid] }]
    const device = await navigator.bluetooth.requestDevice({
      filters,
      optionalServices: [serviceUuid],
    })
    const server = await device.gatt!.connect()
    const service = await server.getPrimaryService(serviceUuid)
    const txChar = await service.getCharacteristic(txUuid)
    const rxChar = await service.getCharacteristic(rxUuid)
    await txChar.startNotifications()
    return new BleTransport(device, txChar, rxChar)
  }

  async send(payload: Uint8Array): Promise<void> {
    await this.rxChar.writeValue(encode(payload))
  }

  async close(): Promise<void> {
    this.queue.close()
    this.device.gatt?.disconnect()
  }

  async *[Symbol.asyncIterator](): AsyncGenerator<Uint8Array> {
    while (true) {
      const frame = await this.queue.pop()
      if (frame === null) break
      yield frame
    }
  }
}
