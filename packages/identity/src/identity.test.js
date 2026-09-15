/**
 * Tests for @fiducial/identity.
 *
 * The important ones are not hand-written assertions — they are
 * `docs/identity/vectors.json`, the decision table generated from the Rust
 * `fiducial-identity` crate. Every probe in it is replayed through this
 * implementation and must reach the same verdict. A divergence in an
 * authorization rule does not look like a bug, it looks like access, so the
 * two implementations are pinned to one table rather than trusted to agree.
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { dirname, join } from 'node:path'
import {
  can,
  effectiveRole,
  isIdentified,
  roleAllows,
  userIdFromUuid,
  userPrincipal,
  devicePrincipal,
  servicePrincipal,
  ANONYMOUS,
} from '../dist/index.js'

const here = dirname(fileURLToPath(import.meta.url))
const vectors = JSON.parse(
  readFileSync(join(here, '../../../docs/identity/vectors.json'), 'utf8'),
)

describe('@fiducial/identity', () => {
  describe('conformance with the Rust rule', () => {
    it('the vectors file is present and populated', () => {
      assert.ok(vectors.scenarios.length > 0, 'scenarios present')
      const probes = vectors.scenarios.reduce((n, s) => n + s.probes.length, 0)
      assert.ok(probes > 50, `enough probes to be meaningful, got ${probes}`)
    })

    for (const scenario of vectors.scenarios) {
      describe(`${scenario.name} — ${scenario.why}`, () => {
        for (const probe of scenario.probes) {
          const label =
            `${probe.principal.kind}${probe.principal.id ? ':' + probe.principal.id.slice(0, 4) : ''}` +
            ` ${probe.action} ` +
            `${probe.resource.kind}${probe.resource.id ? ':' + probe.resource.id.slice(0, 4) : ''}` +
            ` → ${probe.allowed}`

          it(label, () => {
            assert.equal(
              can(probe.principal, probe.action, probe.resource, scenario.grants),
              probe.allowed,
              'verdict must match the Rust implementation',
            )
            assert.equal(
              effectiveRole(probe.principal, probe.resource, scenario.grants),
              probe.effective_role ?? null,
              'effective role must match the Rust implementation',
            )
          })
        }
      })
    }
  })

  describe('roleAllows', () => {
    it('a role permits every action at or below its level', () => {
      assert.equal(roleAllows('viewer', 'read'), true)
      assert.equal(roleAllows('viewer', 'write'), false)
      assert.equal(roleAllows('member', 'write'), true)
      assert.equal(roleAllows('member', 'admin'), false)
      assert.equal(roleAllows('admin', 'admin'), true)
      assert.equal(roleAllows('owner', 'admin'), true)
    })
  })

  describe('isIdentified', () => {
    it('anonymous is never identified', () => {
      assert.equal(isIdentified(ANONYMOUS), false)
    })

    it('an all-zero sentinel id is not an identity', () => {
      assert.equal(isIdentified({ kind: 'device', id: '0000000000000000' }), false)
      assert.equal(
        isIdentified({ kind: 'user', id: '0'.repeat(32) }),
        false,
      )
    })

    it('a real id is', () => {
      assert.equal(isIdentified({ kind: 'device', id: 'dadadadadadadada' }), true)
    })
  })

  describe('userIdFromUuid', () => {
    it('drops hyphens and lowercases', () => {
      assert.equal(
        userIdFromUuid('11111111-2222-3333-4444-555555555555'),
        '11111111222233334444555555555555',
      )
      assert.equal(
        userIdFromUuid('AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE'),
        'aaaaaaaabbbbccccddddeeeeeeeeeeee',
      )
    })

    it('one identity, whatever case the vendor sent', () => {
      assert.equal(
        userIdFromUuid('AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE'),
        userIdFromUuid('aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee'),
      )
    })

    it('rejects junk', () => {
      assert.throws(() => userIdFromUuid('nope'))
      assert.throws(() => userIdFromUuid(''))
      assert.throws(() => userIdFromUuid('11111111-2222-3333-4444-55555555555'))
    })
  })

  describe('principal constructors', () => {
    it('userPrincipal normalizes the vendor UUID', () => {
      assert.deepEqual(userPrincipal('11111111-2222-3333-4444-555555555555'), {
        kind: 'user',
        id: '11111111222233334444555555555555',
      })
    })

    it('devicePrincipal and servicePrincipal lowercase their ids', () => {
      assert.deepEqual(devicePrincipal('DADADADADADADADA'), {
        kind: 'device',
        id: 'dadadadadadadada',
      })
      assert.deepEqual(servicePrincipal('5C5C5C5C5C5C5C5C'), {
        kind: 'service',
        id: '5c5c5c5c5c5c5c5c',
      })
    })
  })

  describe('the rule, stated directly', () => {
    const alice = { kind: 'user', id: 'a'.repeat(32) }
    const devA = { kind: 'device', id: 'dadadadadadadada' }
    const rDevA = { kind: 'device', id: 'dadadadadadadada' }

    it('denies by default', () => {
      assert.equal(can(alice, 'read', rDevA, []), false)
    })

    it('a device may read itself but not write itself', () => {
      assert.equal(can(devA, 'read', rDevA, []), true)
      assert.equal(can(devA, 'write', rDevA, []), false)
    })

    it('an owner may administer', () => {
      const grants = [{ principal: alice, resource: rDevA, role: 'owner' }]
      assert.equal(can(alice, 'admin', rDevA, grants), true)
    })
  })
})
