// node --experimental-strip-types --test web/src/reading.test.ts
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { decode, show } from './reading.ts'
import { READING_BANDS } from '../../protocol/messages.ts'

test('the firmware layout decodes to degrees and percent', () => {
  // −12.34 °C, 48.10 %RH, as firmware/shared's round-trip test writes them.
  const t = new Uint8Array(new Int16Array([-1234]).buffer)
  const h = new Uint8Array(new Uint16Array([4810]).buffer)
  assert.deepEqual(decode(new Uint8Array([1, t[0], t[1], h[0], h[1]])), { celsius: -12.34, percentRh: 48.1 })
})

test('another kind of message is not a reading', () => {
  assert.equal(decode(new Uint8Array([2, 0, 0, 0, 0])), null)
  assert.equal(decode(new Uint8Array([1, 0, 0])), null)
})

test('the page shows a reading in its declared units, and its band', () => {
  assert.equal(show({ celsius: -12.34, percentRh: 48.1 }), '-12.3 °C · 48 %RH · comfortable')
  assert.equal(show({ celsius: 21.5, percentRh: 72 }), '21.5 °C · 72 %RH')
})

test('a bound is inside the band, as it is for the LED', () => {
  const { min, max } = READING_BANDS.comfortable // fid: allow-band (the test reads the declared edges)
  assert.match(show({ celsius: 20, percentRh: min }), /comfortable$/)
  assert.match(show({ celsius: 20, percentRh: max }), /comfortable$/)
  assert.doesNotMatch(show({ celsius: 20, percentRh: max + 0.01 }), /comfortable$/)
})
