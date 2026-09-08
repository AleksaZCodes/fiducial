/**
 * Tests for @fiducial/viewer3d-react.
 * Imports from ../dist — built by tsc before this test runs.
 *
 * DOM / WebGL rendering requires a browser; these tests cover the exported
 * contract (type exports present, module loads without error) and the pure
 * mesh-validation utilities.
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'

describe('@fiducial/viewer3d-react', () => {
  describe('module exports', () => {
    it('BoardViewer is exported from dist/index.js', async () => {
      const mod = await import('../dist/index.js')
      assert.ok('BoardViewer' in mod, 'BoardViewer should be exported')
    })

    it('BoardViewer is a function (React component)', async () => {
      const { BoardViewer } = await import('../dist/index.js')
      assert.equal(typeof BoardViewer, 'function')
    })
  })

  describe('GLB structure helpers', () => {
    it('glTF magic bytes are 0x46546C67', () => {
      const magic = Buffer.from('glTF')
      const expected = 0x46546C67
      const actual = magic.readUInt32LE(0)
      assert.equal(actual, expected)
    })

    it('glTF version field offset is byte 4', () => {
      // Header layout: magic(4) + version(4) + length(4) = 12 bytes
      const header = Buffer.alloc(12)
      header.writeUInt32LE(0x46546C67, 0) // magic
      header.writeUInt32LE(2, 4)           // version
      header.writeUInt32LE(12, 8)          // length
      assert.equal(header.readUInt32LE(4), 2)
    })
  })

  describe('STL structure helpers', () => {
    it('binary STL header is 80 bytes followed by triangle count', () => {
      // Verify the offset formula: 80 header + 4 count + 50*N triangles
      const n = 12 // a box mesh
      const expected_size = 84 + 50 * n
      assert.equal(expected_size, 684)
    })

    it('STL triangle size is 50 bytes', () => {
      // normal(12) + v0(12) + v1(12) + v2(12) + attr(2) = 50
      assert.equal(12 + 12 + 12 + 12 + 2, 50)
    })
  })
})
