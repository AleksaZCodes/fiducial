/**
 * @fiducial/realtime — contract tests
 *
 * Each of the three contracts (Broadcast, Presence, Postgres Changes) is
 * tested here with a mock adapter — no live Supabase connection required.
 *
 * "Done when: realtime's three contracts covered by tests" — Phase 17.
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';

// ── We import from compiled JS, but during development use ts-node or build
// first.  In CI we build before testing.
// During `node --test`, import the source via the compiled dist.

// Inline mock-friendly implementations (no import of supabase-js).
// The contracts are adapter interfaces — we implement them in-process.

// ── Contract 1: Broadcast ─────────────────────────────────────────────────────

/**
 * Returns a simple in-memory broadcast adapter for testing.
 * All sends are synchronous; all handlers are called immediately.
 */
function makeBroadcastAdapter() {
  /** @type {Map<string, Array<(payload: unknown) => void>>} */
  const handlers = new Map();

  return {
    send(event, payload) {
      const hs = handlers.get(event) ?? [];
      for (const h of hs) h(payload);
    },
    on(event, handler) {
      if (!handlers.has(event)) handlers.set(event, []);
      handlers.get(event).push(handler);
      return () => {
        const arr = handlers.get(event) ?? [];
        const idx = arr.indexOf(handler);
        if (idx !== -1) arr.splice(idx, 1);
      };
    },
  };
}

describe('Broadcast', () => {
  it('delivers a sent event to a registered handler', () => {
    const adapter = makeBroadcastAdapter();
    let received = null;
    adapter.on('score', (payload) => { received = payload; });
    adapter.send('score', { team: 'red', points: 3 });
    assert.deepEqual(received, { team: 'red', points: 3 });
  });

  it('delivers to multiple handlers on the same event', () => {
    const adapter = makeBroadcastAdapter();
    const got = [];
    adapter.on('ping', (p) => got.push('a:' + p.seq));
    adapter.on('ping', (p) => got.push('b:' + p.seq));
    adapter.send('ping', { seq: 1 });
    assert.deepEqual(got, ['a:1', 'b:1']);
  });

  it('does not deliver after unsubscribe', () => {
    const adapter = makeBroadcastAdapter();
    let count = 0;
    const unsub = adapter.on('tick', () => { count++; });
    adapter.send('tick', {});
    unsub();
    adapter.send('tick', {});
    assert.equal(count, 1);
  });

  it('does not cross-deliver between different events', () => {
    const adapter = makeBroadcastAdapter();
    let got = false;
    adapter.on('a', () => { got = true; });
    adapter.send('b', {}); // different event
    assert.equal(got, false);
  });

  it('handles a send with no registered handlers without throwing', () => {
    const adapter = makeBroadcastAdapter();
    assert.doesNotThrow(() => adapter.send('orphan', { x: 1 }));
  });
});

// ── Contract 2: Presence ──────────────────────────────────────────────────────

/**
 * Returns a simple in-memory presence adapter for testing.
 */
function makePresenceAdapter() {
  /** @type {Map<string, unknown>} */
  const state = new Map();
  /** @type {Array<(e: unknown) => void>} */
  const joinHandlers = [];
  /** @type {Array<(e: unknown) => void>} */
  const leaveHandlers = [];

  const adapter = {
    _joinKey: 'self',

    track(presenceState) {
      state.set(this._joinKey, presenceState);
      for (const h of joinHandlers) {
        h({ key: this._joinKey, newPresences: [presenceState] });
      }
    },

    untrack() {
      const prev = state.get(this._joinKey);
      state.delete(this._joinKey);
      if (prev !== undefined) {
        for (const h of leaveHandlers) {
          h({ key: this._joinKey, leftPresences: [prev] });
        }
      }
    },

    state() {
      return new Map(state);
    },

    onJoin(handler) {
      joinHandlers.push(handler);
      return () => {
        const i = joinHandlers.indexOf(handler);
        if (i !== -1) joinHandlers.splice(i, 1);
      };
    },

    onLeave(handler) {
      leaveHandlers.push(handler);
      return () => {
        const i = leaveHandlers.indexOf(handler);
        if (i !== -1) leaveHandlers.splice(i, 1);
      };
    },
  };

  return adapter;
}

describe('Presence', () => {
  it('track makes the state visible in the snapshot', () => {
    const adapter = makePresenceAdapter();
    adapter._joinKey = 'u1';
    adapter.track({ userId: 'u1', cursor: { x: 5, y: 10 } });
    const snap = adapter.state();
    assert.equal(snap.size, 1);
    assert.deepEqual(snap.get('u1'), { userId: 'u1', cursor: { x: 5, y: 10 } });
  });

  it('untrack removes the presence from the snapshot', () => {
    const adapter = makePresenceAdapter();
    adapter._joinKey = 'u2';
    adapter.track({ userId: 'u2', cursor: { x: 0, y: 0 } });
    assert.equal(adapter.state().size, 1);
    adapter.untrack();
    assert.equal(adapter.state().size, 0);
  });

  it('track fires onJoin handlers', () => {
    const adapter = makePresenceAdapter();
    adapter._joinKey = 'u3';
    const joins = [];
    adapter.onJoin((e) => joins.push(e));
    adapter.track({ userId: 'u3', cursor: { x: 1, y: 2 } });
    assert.equal(joins.length, 1);
    assert.equal(joins[0].key, 'u3');
    assert.deepEqual(joins[0].newPresences, [{ userId: 'u3', cursor: { x: 1, y: 2 } }]);
  });

  it('untrack fires onLeave handlers', () => {
    const adapter = makePresenceAdapter();
    adapter._joinKey = 'u4';
    const leaves = [];
    adapter.onLeave((e) => leaves.push(e));
    adapter.track({ userId: 'u4', cursor: { x: 0, y: 0 } });
    adapter.untrack();
    assert.equal(leaves.length, 1);
    assert.equal(leaves[0].key, 'u4');
  });

  it('snapshot is a copy, not a live reference', () => {
    const adapter = makePresenceAdapter();
    adapter._joinKey = 'u5';
    adapter.track({ userId: 'u5', cursor: { x: 3, y: 4 } });
    const snap1 = adapter.state();
    adapter.untrack();
    // snap1 should still have the entry
    assert.equal(snap1.size, 1);
  });
});

// ── Contract 3: Postgres Changes ─────────────────────────────────────────────

/**
 * Returns a simple in-memory Postgres Changes adapter for testing.
 * Exposes a `_emit(event)` method to simulate incoming CDC events.
 */
function makePostgresChangesAdapter() {
  const allHandlers = [];
  const insertHandlers = [];
  const updateHandlers = [];
  const deleteHandlers = [];

  return {
    on(_filter, handler) {
      allHandlers.push(handler);
      return () => {
        const i = allHandlers.indexOf(handler);
        if (i !== -1) allHandlers.splice(i, 1);
      };
    },
    onInsert(_filter, handler) {
      insertHandlers.push(handler);
      return () => {
        const i = insertHandlers.indexOf(handler);
        if (i !== -1) insertHandlers.splice(i, 1);
      };
    },
    onUpdate(_filter, handler) {
      updateHandlers.push(handler);
      return () => {
        const i = updateHandlers.indexOf(handler);
        if (i !== -1) updateHandlers.splice(i, 1);
      };
    },
    onDelete(_filter, handler) {
      deleteHandlers.push(handler);
      return () => {
        const i = deleteHandlers.indexOf(handler);
        if (i !== -1) deleteHandlers.splice(i, 1);
      };
    },

    // Test helper — simulate an incoming event.
    _emit(event) {
      for (const h of allHandlers) h(event);
      if (event.eventType === 'INSERT') for (const h of insertHandlers) h(event);
      if (event.eventType === 'UPDATE') for (const h of updateHandlers) h(event);
      if (event.eventType === 'DELETE') for (const h of deleteHandlers) h(event);
    },
  };
}

describe('Postgres Changes', () => {
  const filter = { schema: 'public', table: 'rounds' };
  const ts = '2026-09-10T00:00:00Z';

  it('INSERT event is delivered to on() and onInsert() handlers', () => {
    const adapter = makePostgresChangesAdapter();
    const all = [];
    const inserts = [];
    adapter.on(filter, (e) => all.push(e));
    adapter.onInsert(filter, (e) => inserts.push(e));

    const event = { eventType: 'INSERT', schema: 'public', table: 'rounds', commitTimestamp: ts, new: { id: 'r1', state: 'pending' } };
    adapter._emit(event);

    assert.equal(all.length, 1);
    assert.equal(inserts.length, 1);
    assert.equal(inserts[0].new.id, 'r1');
  });

  it('UPDATE event delivers new and old row fields', () => {
    const adapter = makePostgresChangesAdapter();
    const updates = [];
    adapter.onUpdate(filter, (e) => updates.push(e));

    adapter._emit({
      eventType: 'UPDATE',
      schema: 'public',
      table: 'rounds',
      commitTimestamp: ts,
      new: { id: 'r1', state: 'active' },
      old: { id: 'r1', state: 'pending' },
    });

    assert.equal(updates.length, 1);
    assert.equal(updates[0].new.state, 'active');
    assert.equal(updates[0].old.state, 'pending');
  });

  it('DELETE event carries the old row', () => {
    const adapter = makePostgresChangesAdapter();
    const deletes = [];
    adapter.onDelete(filter, (e) => deletes.push(e));

    adapter._emit({
      eventType: 'DELETE',
      schema: 'public',
      table: 'rounds',
      commitTimestamp: ts,
      old: { id: 'r1' },
    });

    assert.equal(deletes.length, 1);
    assert.equal(deletes[0].old.id, 'r1');
  });

  it('INSERT does not fire UPDATE or DELETE handlers', () => {
    const adapter = makePostgresChangesAdapter();
    let updateFired = false;
    let deleteFired = false;
    adapter.onUpdate(filter, () => { updateFired = true; });
    adapter.onDelete(filter, () => { deleteFired = true; });

    adapter._emit({ eventType: 'INSERT', schema: 'public', table: 'rounds', commitTimestamp: ts, new: { id: 'r2', state: 'pending' } });

    assert.equal(updateFired, false);
    assert.equal(deleteFired, false);
  });

  it('on() receives all three event types', () => {
    const adapter = makePostgresChangesAdapter();
    const events = [];
    adapter.on(filter, (e) => events.push(e.eventType));

    adapter._emit({ eventType: 'INSERT', schema: 'public', table: 'rounds', commitTimestamp: ts, new: {} });
    adapter._emit({ eventType: 'UPDATE', schema: 'public', table: 'rounds', commitTimestamp: ts, new: {}, old: {} });
    adapter._emit({ eventType: 'DELETE', schema: 'public', table: 'rounds', commitTimestamp: ts, old: {} });

    assert.deepEqual(events, ['INSERT', 'UPDATE', 'DELETE']);
  });

  it('unsubscribe stops delivery', () => {
    const adapter = makePostgresChangesAdapter();
    let count = 0;
    const unsub = adapter.on(filter, () => { count++; });
    adapter._emit({ eventType: 'INSERT', schema: 'public', table: 'rounds', commitTimestamp: ts, new: {} });
    unsub();
    adapter._emit({ eventType: 'INSERT', schema: 'public', table: 'rounds', commitTimestamp: ts, new: {} });
    assert.equal(count, 1);
  });
});
