/**
 * Minimal type stubs for experimental browser APIs not yet in TypeScript's DOM lib.
 * Covers exactly what @fiducial/transport-web uses — no more.
 */

// ── Web Serial API ────────────────────────────────────────────────────────────

interface SerialPortFilter {
  usbVendorId?: number
  usbProductId?: number
}

interface SerialOptions {
  baudRate: number
  dataBits?: number
  stopBits?: number
  parity?: string
  bufferSize?: number
  flowControl?: string
}

interface SerialPort {
  readonly readable: ReadableStream<Uint8Array> | null
  readonly writable: WritableStream<Uint8Array> | null
  open(options: SerialOptions): Promise<void>
  close(): Promise<void>
}

interface Serial {
  requestPort(options?: { filters?: SerialPortFilter[] }): Promise<SerialPort>
}

interface Navigator {
  readonly serial: Serial
}

// ── WebUSB API ────────────────────────────────────────────────────────────────

interface USBDeviceFilter {
  vendorId?: number
  productId?: number
  classCode?: number
  subclassCode?: number
  protocolCode?: number
  serialNumber?: string
}

interface USBInTransferResult {
  readonly data: DataView | null
  readonly status: 'ok' | 'stall' | 'babble'
}

interface USBDevice {
  open(): Promise<void>
  close(): Promise<void>
  selectConfiguration(configurationValue: number): Promise<void>
  claimInterface(interfaceNumber: number): Promise<void>
  releaseInterface(interfaceNumber: number): Promise<void>
  transferOut(endpointNumber: number, data: AllowSharedBufferSource): Promise<{ bytesWritten: number; status: string }>
  transferIn(endpointNumber: number, length: number): Promise<USBInTransferResult>
}

interface USB {
  requestDevice(options: { filters: USBDeviceFilter[] }): Promise<USBDevice>
}

interface Navigator {
  readonly usb: USB
}

// ── Web Bluetooth API ─────────────────────────────────────────────────────────

interface BluetoothLEScanFilter {
  services?: string[]
  name?: string
  namePrefix?: string
}

interface BluetoothRemoteGATTCharacteristic extends EventTarget {
  readonly value: DataView | null
  startNotifications(): Promise<BluetoothRemoteGATTCharacteristic>
  stopNotifications(): Promise<BluetoothRemoteGATTCharacteristic>
  writeValue(value: AllowSharedBufferSource): Promise<void>
}

interface BluetoothRemoteGATTService {
  getCharacteristic(characteristic: string): Promise<BluetoothRemoteGATTCharacteristic>
}

interface BluetoothRemoteGATTServer {
  readonly connected: boolean
  connect(): Promise<BluetoothRemoteGATTServer>
  disconnect(): void
  getPrimaryService(service: string): Promise<BluetoothRemoteGATTService>
}

interface BluetoothDevice extends EventTarget {
  readonly gatt: BluetoothRemoteGATTServer | null
}

interface Bluetooth {
  requestDevice(options: {
    filters?: BluetoothLEScanFilter[]
    optionalServices?: string[]
  }): Promise<BluetoothDevice>
}

interface Navigator {
  readonly bluetooth: Bluetooth
}
