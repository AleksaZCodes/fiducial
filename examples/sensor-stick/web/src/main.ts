// The sensor stick's page: connect over Web Serial, show each reading.
//
// The pin names and the socket come from hardware/generated/interface.ts,
// which `fid derive` writes from hardware/product.toml — the same
// declaration the board and the firmware are built from. Rename the sensor's
// net there and this file stops type-checking until it follows.

import { WebSerialTransport } from '@fiducial/transport-web'
import { CONNECTORS, MCU_PINS } from '../../hardware/generated/interface.ts'
import { decode, show } from './reading.ts'

const status = document.querySelector<HTMLParagraphElement>('#status')!
const value = document.querySelector<HTMLParagraphElement>('#value')!
const button = document.querySelector<HTMLButtonElement>('#connect')!

const socket = CONNECTORS.find((c) => c.part === 'usb')
document.querySelector('#about')!.textContent =
  `AHT20 on ${MCU_PINS.SENSOR_SDA} / ${MCU_PINS.SENSOR_SCL}; plug in at the ${socket?.side ?? ''} end.`

button.addEventListener('click', async () => {
  button.disabled = true
  try {
    const stick = await WebSerialTransport.open({ filters: [{ usbVendorId: 0x2e8a }] })
    status.textContent = 'Connected.'
    for await (const payload of stick) {
      const r = decode(payload)
      if (r) value.textContent = show(r)
    }
  } catch (e) {
    status.textContent = `Not connected: ${(e as Error).message}`
    button.disabled = false
  }
})
