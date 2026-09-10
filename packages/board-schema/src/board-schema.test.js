/**
 * Tests for @fiducial/board-schema.
 * Imports from ../dist — built by tsc before this test runs (turbo: test dependsOn build).
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { join, dirname } from 'node:path'
import { parseBoardInterface, mountEnvelope, CONNECTOR_OPENINGS } from '../dist/index.js'

const __dirname = dirname(fileURLToPath(import.meta.url))

// Load the canonical seed file from the EDA capability.
const SEED_PATH = join(
  __dirname,
  '../../../crates/fiducial-cli/capabilities/eda/board/board.interface.json'
)
const SEED_JSON = readFileSync(SEED_PATH, 'utf8')

describe('@fiducial/board-schema', () => {
  describe('parseBoardInterface()', () => {
    it('parses the seed board.interface.json without error', () => {
      assert.doesNotThrow(() => parseBoardInterface(SEED_JSON))
    })

    it('returns schema_version "1.0"', () => {
      const bi = parseBoardInterface(SEED_JSON)
      assert.equal(bi.schema_version, '1.0')
    })

    it('board.name is a non-empty string', () => {
      const bi = parseBoardInterface(SEED_JSON)
      assert.equal(typeof bi.board.name, 'string')
      assert.ok(bi.board.name.length > 0)
    })

    it('connectors is a non-empty array', () => {
      const bi = parseBoardInterface(SEED_JSON)
      assert.ok(Array.isArray(bi.connectors))
      assert.ok(bi.connectors.length > 0)
    })

    it('each connector has a non-empty pins array', () => {
      const bi = parseBoardInterface(SEED_JSON)
      for (const c of bi.connectors) {
        assert.ok(Array.isArray(c.pins), `${c.id} pins is not an array`)
        assert.ok(c.pins.length > 0, `${c.id} has no pins`)
      }
    })

    it('all pins have a valid direction', () => {
      const bi = parseBoardInterface(SEED_JSON)
      const validDirections = new Set(['input', 'output', 'bidirectional', 'power_in', 'power_out'])
      for (const c of bi.connectors) {
        for (const pin of c.pins) {
          assert.ok(
            validDirections.has(pin.direction),
            `${c.id}.pin${pin.number} has invalid direction: ${pin.direction}`
          )
        }
      }
    })

    it('net_classes is an array', () => {
      const bi = parseBoardInterface(SEED_JSON)
      assert.ok(Array.isArray(bi.net_classes))
      assert.ok(bi.net_classes.length > 0)
    })

    it('throws on malformed JSON', () => {
      assert.throws(() => parseBoardInterface('{not json}'))
    })

    it('throws on wrong schema_version', () => {
      const bad = JSON.stringify({
        schema_version: '2.0',
        board: { name: 'x' },
        connectors: [],
        net_classes: [],
      })
      assert.throws(() => parseBoardInterface(bad), /unsupported schema_version/)
    })

    it('throws when board.name is missing', () => {
      const bad = JSON.stringify({
        schema_version: '1.0',
        board: {},
        connectors: [],
        net_classes: [],
      })
      assert.throws(() => parseBoardInterface(bad), /board\.name/)
    })

    it('optional board fields (revision, description) are preserved', () => {
      const bi = parseBoardInterface(SEED_JSON)
      assert.equal(typeof bi.board.revision, 'string')
      assert.equal(typeof bi.board.description, 'string')
    })
  })

  describe('outline', () => {
    const withOutline = outline =>
      JSON.stringify({
        schema_version: '1.0',
        board: { name: 'x' },
        outline,
        connectors: [],
        net_classes: [],
      })

    it('reads the outline declared by the seed board', () => {
      const { outline } = parseBoardInterface(SEED_JSON)
      assert.ok(outline, 'seed must declare an outline')
      assert.ok(outline.width_mm > 0)
      assert.ok(outline.height_mm > 0)
      assert.equal(outline.tolerance, 'fdm')
    })

    it('treats outline as optional', () => {
      const json = JSON.stringify({
        schema_version: '1.0',
        board: { name: 'x' },
        connectors: [],
        net_classes: [],
      })
      assert.equal(parseBoardInterface(json).outline, undefined)
    })

    it('rejects a non-positive dimension', () => {
      assert.throws(
        () => parseBoardInterface(withOutline({ width_mm: 0, height_mm: 5 })),
        /must all be > 0/
      )
    })

    it('rejects an unknown tolerance class', () => {
      assert.throws(
        () => parseBoardInterface(withOutline({ width_mm: 10, height_mm: 5, tolerance: 'sintering' })),
        /unknown tolerance/
      )
    })

    it('accepts each supported tolerance class', () => {
      for (const tolerance of ['fdm', 'resin', 'cnc']) {
        assert.doesNotThrow(() =>
          parseBoardInterface(withOutline({ width_mm: 10, height_mm: 5, tolerance }))
        )
      }
    })
  })

  describe('outline.enclosure', () => {
    const withEnclosure = enclosure =>
      JSON.stringify({
        schema_version: '1.0',
        board: { name: 'x' },
        outline: { width_mm: 10, height_mm: 5, enclosure },
        connectors: [],
        net_classes: [],
      })

    it('is optional', () => {
      const json = JSON.stringify({
        schema_version: '1.0',
        board: { name: 'x' },
        outline: { width_mm: 10, height_mm: 5 },
        connectors: [],
        net_classes: [],
      })
      assert.equal(parseBoardInterface(json).outline.enclosure, undefined)
    })

    it('accepts partial overrides', () => {
      const bi = parseBoardInterface(withEnclosure({ headroom_mm: 15 }))
      assert.equal(bi.outline.enclosure.headroom_mm, 15)
      assert.equal(bi.outline.enclosure.gasket_width_mm, undefined)
    })

    it('rejects a non-positive dimension', () => {
      assert.throws(() => parseBoardInterface(withEnclosure({ headroom_mm: 0 })), /must be > 0/)
      assert.throws(
        () => parseBoardInterface(withEnclosure({ gasket_height_mm: -1 })),
        /must be > 0/
      )
    })

    it('rejects gasket compression outside (0, 1)', () => {
      // 0 never squeezes the gasket; 1 crushes it flat. Neither seals.
      for (const c of [0, 1, 1.5, -0.2]) {
        assert.throws(
          () => parseBoardInterface(withEnclosure({ gasket_compression: c })),
          /gasket_compression/
        )
      }
    })

    it('accepts a sane compression fraction', () => {
      assert.doesNotThrow(() => parseBoardInterface(withEnclosure({ gasket_compression: 0.25 })))
    })
  })
  describe('connector mounts', () => {
    const withMount = mount =>
      JSON.stringify({
        schema_version: '1.0',
        board: { name: 'x' },
        outline: { width_mm: 100, height_mm: 60 },
        connectors: [
          {
            id: 'J1',
            name: 'USB',
            type: 'usb-c',
            pins: [{ number: 1, name: 'VBUS', net: 'V', direction: 'power_in' }],
            mount,
          },
        ],
        net_classes: [],
      })

    it('reads the mount the seed declares for its USB port', () => {
      const bi = parseBoardInterface(SEED_JSON)
      const j1 = bi.connectors.find(c => c.id === 'J1')
      assert.equal(j1.mount.side, 'south')
      assert.equal(j1.mount.width_mm, undefined, 'size comes from the family')
    })

    it('leaves the seed debug header deliberately unmounted', () => {
      // A 2x3 shrouded header is 8.5 mm tall, so mounting it needs more than
      // 11.7 mm of headroom on the FDM profile — more than the seed declares.
      // It is reached with the lid off instead.
      const bi = parseBoardInterface(SEED_JSON)
      assert.equal(bi.connectors.find(c => c.id === 'J2').mount, undefined)
    })

    it('treats mount as optional', () => {
      const bi = parseBoardInterface(SEED_JSON)
      assert.ok(bi.connectors.some(c => c.mount))
      assert.ok(bi.connectors.some(c => !c.mount))
    })

    it('falls back to the connector family envelope', () => {
      assert.deepEqual(mountEnvelope({ side: 'south', offset_mm: 20 }, 'usb-c'), {
        width_mm: 8.94,
        height_mm: 3.26,
      })
    })

    it('prefers a declared envelope over the family default', () => {
      assert.deepEqual(
        mountEnvelope({ side: 'south', offset_mm: 20, width_mm: 12, height_mm: 4 }, 'usb-c'),
        { width_mm: 12, height_mm: 4 }
      )
    })

    it('returns undefined for a family it cannot size', () => {
      assert.equal(mountEnvelope({ side: 'south', offset_mm: 20 }, 'db25'), undefined)
    })

    it('rejects an unknown side', () => {
      assert.throws(
        () => parseBoardInterface(withMount({ side: 'starboard', offset_mm: 20 })),
        /unknown side/
      )
    })

    it('rejects a negative offset', () => {
      assert.throws(
        () => parseBoardInterface(withMount({ side: 'south', offset_mm: -5 })),
        /offset_mm/
      )
    })

    it('rejects a non-positive mount dimension', () => {
      assert.throws(
        () => parseBoardInterface(withMount({ side: 'south', offset_mm: 20, width_mm: 0 })),
        /width_mm/
      )
    })

    it('rejects a mount with no outline to measure against', () => {
      const json = JSON.stringify({
        schema_version: '1.0',
        board: { name: 'x' },
        connectors: [
          {
            id: 'J1',
            name: 'USB',
            type: 'usb-c',
            pins: [],
            mount: { side: 'south', offset_mm: 20 },
          },
        ],
        net_classes: [],
      })
      assert.throws(() => parseBoardInterface(json), /no outline/)
    })

    it('rejects a connector family it cannot size', () => {
      const json = JSON.stringify({
        schema_version: '1.0',
        board: { name: 'x' },
        outline: { width_mm: 100, height_mm: 60 },
        connectors: [
          {
            id: 'J7',
            name: 'Mystery',
            type: 'db25',
            pins: [],
            mount: { side: 'south', offset_mm: 20 },
          },
        ],
        net_classes: [],
      })
      assert.throws(() => parseBoardInterface(json), /db25/)
    })

    it('every family envelope is printable', () => {
      for (const [name, o] of Object.entries(CONNECTOR_OPENINGS)) {
        assert.ok(o.width_mm > 0, `${name} width`)
        assert.ok(o.height_mm > 0, `${name} height`)
      }
    })
  })

  describe('standoffs', () => {
    it('reads the standoffs the seed declares', () => {
      const e = parseBoardInterface(SEED_JSON).outline.enclosure
      assert.equal(e.standoff_height_mm, 3)
      assert.equal(e.standoff_size_mm, 5)
    })

    it('rejects a non-positive standoff height', () => {
      const json = JSON.stringify({
        schema_version: '1.0',
        board: { name: 'x' },
        outline: { width_mm: 100, height_mm: 60, enclosure: { standoff_height_mm: 0 } },
        connectors: [],
        net_classes: [],
      })
      assert.throws(() => parseBoardInterface(json), /standoff_height_mm/)
    })
  })
})
