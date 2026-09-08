/**
 * Tests for @fiducial/board-schema.
 * Imports from ../dist — built by tsc before this test runs (turbo: test dependsOn build).
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { join, dirname } from 'node:path'
import { parseBoardInterface } from '../dist/index.js'

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
})
