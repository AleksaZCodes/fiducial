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
  DEFAULT_BACKOFF_SCHEDULE_MS,
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

      it('defaults maxRetries to the length of the backoff schedule', () => {
        // Was a hardcoded 3. Changed in Phase 20, when the backoff schedule was
        // harvested from Ring of Pursuit: a retry COUNT and a retry SCHEDULE
        // are two declarations of the same fact, and 3 retries against a
        // 5-step schedule made the last two delays unreachable. The count now
        // derives from the schedule, so they cannot disagree.
        const q = new OfflineQueue()
        const action = q.enqueue('a', 'x')

        let attempts = 0
        while (q.retry(action)) {
          attempts += 1
          q.dequeue()
        }
        assert.equal(attempts, DEFAULT_BACKOFF_SCHEDULE_MS.length)
      })

      it('still honours an explicit maxRetries', () => {
        const q = new OfflineQueue({ maxRetries: 3 })
        const action = q.enqueue('a', 'x')
        assert.equal(q.retry(action), true)
        assert.equal(q.retry(action), true)
        assert.equal(q.retry(action), true)
        assert.equal(q.retry(action), false)
      })
    })
  })
})

describe('OfflineQueue — durability (harvested from Ring of Pursuit)', () => {
  it('replays actions at their original timestamp, not at sync time', async () => {
    const q = new OfflineQueue()
    const first = q.enqueue('a', { v: 1 })
    await new Promise((r) => setTimeout(r, 5))
    q.enqueue('b', { v: 2 })

    const drained = q.drain()
    assert.equal(drained.length, 2)
    // Order is enqueue order, and each carries the moment it was captured.
    assert.equal(drained[0].id, 'a')
    assert.equal(drained[0].enqueuedAt.getTime(), first.enqueuedAt.getTime())
    assert.ok(drained[1].enqueuedAt.getTime() >= drained[0].enqueuedAt.getTime())
  })

  it('defaults maxRetries to the length of the backoff schedule', () => {
    // A retry count without a matching schedule was the gap in the first
    // version: three retries against a five-step backoff meant the last two
    // delays were unreachable.
    const q = new OfflineQueue()
    const action = q.enqueue('a', 1)
    let attempts = 0
    while (q.retry(action)) attempts += 1
    assert.equal(attempts, DEFAULT_BACKOFF_SCHEDULE_MS.length)
  })

  it('returns the scheduled delay for each attempt, then undefined', () => {
    const q = new OfflineQueue({ backoffScheduleMs: [10, 20, 30] })
    const action = q.enqueue('a', 1)

    assert.equal(q.backoffFor(action), 10)
    q.retry(action)
    assert.equal(q.backoffFor(action), 20)
    q.retry(action)
    assert.equal(q.backoffFor(action), 30)
    q.retry(action)
    // Exhausted — the caller should surface the failure, not keep waiting.
    assert.equal(q.backoffFor(action), undefined)
  })

  it('repeats the final delay when maxRetries exceeds the schedule', () => {
    const q = new OfflineQueue({ backoffScheduleMs: [10, 20], maxRetries: 4 })
    const action = q.enqueue('a', 1)
    q.retry(action)
    q.retry(action)
    q.retry(action)
    assert.equal(q.backoffFor(action), 20)
  })

  it('pending() reports the queue without draining it', () => {
    const q = new OfflineQueue()
    q.enqueue('a', 1)
    q.enqueue('b', 2)

    assert.equal(q.pending().length, 2)
    assert.equal(q.size, 2, 'pending() must not consume')
    // Filterable, which a bare count is not — the donor could only count.
    assert.equal(q.pending().filter((a) => a.payload === 2).length, 1)
  })

  it('survives a reload when given durable storage', () => {
    // The whole point of the second harvest pass: an in-memory queue loses its
    // contents on the page reload that being offline tends to cause.
    const backing = []
    const durable = {
      all: () => [...backing],
      push: (a) => backing.push(a),
      unshift: (a) => backing.unshift(a),
      shift: () => backing.shift(),
      clear: () => {
        backing.length = 0
      },
    }

    const before = new OfflineQueue({ storage: durable })
    before.enqueue('a', { move: 'north' })
    before.enqueue('b', { move: 'south' })

    // A new queue over the same store — this is what a reload looks like.
    const after = new OfflineQueue({ storage: durable })
    assert.equal(after.size, 2)
    assert.equal(after.dequeue().payload.move, 'north')
  })

  it('storage is the only difference between the durable and test paths', () => {
    // Same assertions, both adapters — so the durable path is not a second
    // implementation that can drift from the tested one.
    const backing = []
    const durable = {
      all: () => [...backing],
      push: (a) => backing.push(a),
      unshift: (a) => backing.unshift(a),
      shift: () => backing.shift(),
      clear: () => {
        backing.length = 0
      },
    }

    for (const storage of [undefined, durable]) {
      const q = new OfflineQueue(storage ? { storage } : {})
      q.enqueue('a', 1)
      q.enqueue('b', 2)
      assert.equal(q.size, 2)
      assert.equal(q.dequeue().id, 'a')
      assert.equal(q.drain().length, 1)
      assert.ok(q.isEmpty)
    }
  })
})
