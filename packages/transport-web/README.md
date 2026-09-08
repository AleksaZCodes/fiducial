# @fiducial/transport-web

Browser transports for Fiducial devices — Web Serial, WebUSB, and Web Bluetooth,
all speaking the same frame codec as the firmware.

The codec is a TypeScript port of the `fiducial-protocol` Rust crate and is
byte-exact with it: same magic, same little-endian length, same CRC-8 XOR fold.
A frame written by an RP2040 decodes unchanged in the browser.

```
[ MAGIC(1) | LEN_LO(1) | LEN_HI(1) | PAYLOAD(N) | CRC8(1) ]
```

## Install

```sh
pnpm add @fiducial/transport-web
```

## Usage

All three transports implement the same `Transport` interface — an
`AsyncIterable<Uint8Array>` of decoded payloads, plus `send()` and `close()`.
Swapping transports does not change the code that consumes frames.

```ts
import { WebSerialTransport } from '@fiducial/transport-web'

const transport = await WebSerialTransport.open({ baudRate: 115200 })
await transport.send(new Uint8Array([0x01, 0x02]))

for await (const frame of transport) {
  console.log('payload', frame)
}

await transport.close()
```

WebUSB and BLE follow the same shape:

```ts
import { WebUsbTransport, BleTransport } from '@fiducial/transport-web'

const usb = await WebUsbTransport.open({ filters: [{ vendorId: 0x1209 }] })
const ble = await BleTransport.open()
```

## Payload size

`encode()` accepts payloads up to `MAX_PAYLOAD` (65535 bytes), but each transport
decodes into a fixed buffer that defaults to **4096 bytes**. A frame declaring a
longer payload is dropped silently — the decoder resynchronises rather than
throwing. If your device sends larger frames, raise the cap:

```ts
await WebSerialTransport.open({ maxPayload: 65535 })
```

## BLE UUIDs

The default service and characteristic UUIDs (`fd01`/`fd02`/`fd03`) are
**placeholders** in the 16-bit SIG range and are not allocated to Fiducial. Ship
a BLE product only after substituting your own — changing a UUID after devices
are in the field breaks every existing pairing.

```ts
await BleTransport.open({
  serviceUuid: '...',
  txUuid: '...', // device → browser, notify
  rxUuid: '...', // browser → device, write
})
```

## Browser support

| Transport | Requires |
|---|---|
| `WebSerialTransport` | Web Serial API — Chrome/Edge 89+ |
| `WebUsbTransport` | WebUSB API — Chrome/Edge 61+ |
| `BleTransport` | Web Bluetooth API — Chrome/Edge 56+ |

All three need a secure context (HTTPS or localhost) and must be triggered by a
user gesture — the browser shows a device picker that the page cannot bypass.

## Codec directly

The codec is exported on its own if you are bridging a transport this package
does not cover:

```ts
import { encode, FrameDecoder, crc8, MAGIC, MAX_PAYLOAD } from '@fiducial/transport-web'

const decoder = new FrameDecoder(4096)
for (const byte of incomingBytes) {
  const payload = decoder.feed(byte) // Uint8Array once a valid frame completes
  if (payload) handle(payload)
}
```

`feed()` returns `null` for every byte that does not complete a valid frame,
including on CRC failure — a corrupt frame is dropped and the decoder resyncs on
the next magic byte.

## License

MIT
