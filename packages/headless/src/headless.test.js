/**
 * Tests for @fiducial/headless.
 * Imports from ../dist — built by tsc before this test runs (turbo: test dependsOn build).
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import {
  ok,
  err,
  isOk,
  isErr,
  unwrap,
  map,
  flatMap,
  OfflineQueue,
} from '../dist/index.js'

describe('@fiducial/headless', () => {
  describe('Result', () => {
    describe('ok()', () => {
      it('sets ok: true', () => {
        assert.equal(ok(42).ok, true)
      })
      it('carries the value', () => {
        assert.equal(ok('hello').value, 'hello')
      })
      it('works with any type', () => {
        const r = ok({ x: 1 })
        assert.deepEqual(r.value, { x: 1 })
      })
    })

    describe('err()', () => {
      it('sets ok: false', () => {
        assert.equal(err(new Error('oops')).ok, false)
      })
      it('carries the error', () => {
        assert.equal(err(new Error('oops')).error.message, 'oops')
      })
      it('works with non-Error error types', () => {
        const r = err('string-error')
        assert.equal(r.error, 'string-error')
      })
    })

    describe('isOk() / isErr()', () => {
      it('isOk is true for ok results', () => assert.equal(isOk(ok(1)), true))
      it('isOk is false for err results', () => assert.equal(isOk(err('x')), false))
      it('isErr is true for err results', () => assert.equal(isErr(err('x')), true))
      it('isErr is false for ok results', () => assert.equal(isErr(ok(1)), false))
      it('isOk and isErr are always opposite', () => {
        for (const r of [ok(0), err('e'), ok(null), err(42)]) {
          assert.equal(isOk(r), !isErr(r))
        }
      })
    })

    describe('unwrap()', () => {
      it('returns value on Ok', () => {
        assert.equal(unwrap(ok('hello')), 'hello')
      })
      it('throws the error on Err', () => {
        assert.throws(() => unwrap(err(new Error('bad'))), /bad/)
      })
      it('throws non-Error errors directly', () => {
        assert.throws(() => unwrap(err('oops')), (e) => e === 'oops')
      })
    })

    describe('map()', () => {
      it('transforms the value on Ok', () => {
        assert.equal(unwrap(map(ok(2), x => x * 3)), 6)
      })
      it('passes Err through without calling fn', () => {
        let called = false
        const r = map(err('fail'), () => { called = true; return 1 })
        assert.equal(called, false)
        assert.equal(isErr(r), true)
      })
      it('preserves the error value on Err', () => {
        const original = err('e')
        const result = map(original, x => x)
        assert.equal(result.ok, false)
        if (!result.ok) assert.equal(result.error, 'e')
      })
    })

    describe('flatMap()', () => {
      it('chains two ok results', () => {
        const r = flatMap(ok(5), x => ok(x * 2))
        assert.equal(unwrap(r), 10)
      })
      it('short-circuits on first Err', () => {
        let called = false
        const r = flatMap(err('nope'), () => { called = true; return ok(1) })
        assert.equal(called, false)
        assert.equal(isErr(r), true)
      })
      it('propagates error from the chained fn', () => {
        const r = flatMap(ok(1), () => err('inner'))
        assert.equal(isErr(r), true)
        if (!r.ok) assert.equal(r.error, 'inner')
      })
    })
  })

  describe('OfflineQueue', () => {
    describe('basic operations', () => {
      it('starts empty', () => {
        const q = new OfflineQueue()
        assert.equal(q.size, 0)
        assert.equal(q.isEmpty, true)
      })

      it('enqueue increases size', () => {
        const q = new OfflineQueue()
        q.enqueue('a', { type: 'join' })
        assert.equal(q.size, 1)
        assert.equal(q.isEmpty, false)
      })

      it('dequeue removes and returns the front item', () => {
        const q = new OfflineQueue()
        q.enqueue('a', 1)
        q.enqueue('b', 2)
        const item = q.dequeue()
        assert.equal(item?.id, 'a')
        assert.equal(item?.payload, 1)
        assert.equal(q.size, 1)
      })

      it('dequeue on empty queue returns undefined', () => {
        assert.equal(new OfflineQueue().dequeue(), undefined)
      })

      it('preserves FIFO order', () => {
        const q = new OfflineQueue()
        q.enqueue('first', 1)
        q.enqueue('second', 2)
        q.enqueue('third', 3)
        assert.equal(q.dequeue()?.payload, 1)
        assert.equal(q.dequeue()?.payload, 2)
        assert.equal(q.dequeue()?.payload, 3)
      })
    })

    describe('enqueue()', () => {
      it('sets enqueuedAt to approximately now', () => {
        const before = new Date()
        const q = new OfflineQueue()
        const action = q.enqueue('t', 'payload')
        const after = new Date()
        assert.ok(action.enqueuedAt >= before, 'enqueuedAt should be after before')
        assert.ok(action.enqueuedAt <= after, 'enqueuedAt should be before after')
      })

      it('sets retries to 0', () => {
        const q = new OfflineQueue()
        assert.equal(q.enqueue('a', 'x').retries, 0)
      })

      it('returns the queued action', () => {
        const q = new OfflineQueue()
        const action = q.enqueue('id-42', { value: 7 })
        assert.equal(action.id, 'id-42')
        assert.deepEqual(action.payload, { value: 7 })
      })
    })

    describe('drain()', () => {
      it('removes and returns all items', () => {
        const q = new OfflineQueue()
        q.enqueue('a', 1)
        q.enqueue('b', 2)
        const items = q.drain()
        assert.equal(items.length, 2)
        assert.equal(q.isEmpty, true)
      })

      it('drain on empty queue returns []', () => {
        assert.deepEqual(new OfflineQueue().drain(), [])
      })

      it('preserves order in drained items', () => {
        const q = new OfflineQueue()
        q.enqueue('a', 1)
        q.enqueue('b', 2)
        const [first, second] = q.drain()
        assert.equal(first.id, 'a')
        assert.equal(second.id, 'b')
      })
    })

    describe('retry()', () => {
      it('re-queues the action at the front', () => {
        const q = new OfflineQueue()
        const a = q.enqueue('a', 'first')
        q.enqueue('b', 'second')
        q.dequeue() // remove a
        q.retry(a)
        assert.equal(q.dequeue()?.id, 'a') // a is back at front
      })

      it('increments retries', () => {
        const q = new OfflineQueue({ maxRetries: 3 })
        const action = q.enqueue('a', 'x')
        q.dequeue()
        q.retry(action)
        assert.equal(action.retries, 1)
      })

      it('returns true when retries remain', () => {
        const q = new OfflineQueue({ maxRetries: 2 })
        const action = q.enqueue('a', 'x')
        q.dequeue()
        assert.equal(q.retry(action), true)
      })

      it('returns false when maxRetries is exhausted', () => {
        const q = new OfflineQueue({ maxRetries: 1 })
        const action = q.enqueue('a', 'x')
        q.dequeue()
        q.retry(action)   // retries → 1, re-queued
        q.dequeue()       // remove again
        assert.equal(q.retry(action), false)  // retries (1) >= maxRetries (1)
      })

      it('does not queue the action when maxRetries is exhausted', () => {
        const q = new OfflineQueue({ maxRetries: 0 })
        const action = q.enqueue('a', 'x')
        q.dequeue()
        q.retry(action)
        assert.equal(q.isEmpty, true)
      })

      it('defaults to maxRetries 3', () => {
        const q = new OfflineQueue()
        const action = q.enqueue('a', 'x')
        q.dequeue()
        assert.equal(q.retry(action), true)  // retry 1
        q.dequeue()
        assert.equal(q.retry(action), true)  // retry 2
        q.dequeue()
        assert.equal(q.retry(action), true)  // retry 3
        q.dequeue()
        assert.equal(q.retry(action), false) // exhausted
      })
    })
  })
})
