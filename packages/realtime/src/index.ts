/**
 * @fiducial/realtime — Supabase Realtime typed wrappers.
 *
 * Three contracts, one declaration each:
 *
 * 1. **Broadcast** — fire-and-forget ephemeral messages to all subscribers on a
 *    channel.  No persistence, no history.
 *
 * 2. **Presence** — shared state about who is online.  Tracks join/leave events
 *    and exposes a typed snapshot of the current presence set.
 *
 * 3. **Postgres Changes** — CDC stream from a Postgres table: INSERT, UPDATE,
 *    DELETE.  Arrives with the new row (or old row on DELETE).
 *
 * Each contract is a thin typed layer over whatever channel implementation the
 * caller provides.  The package does **not** import `@supabase/supabase-js`
 * directly — it accepts an adapter interface, so tests run without a live
 * connection and products swap in their own Supabase client.
 */

export type { BroadcastAdapter, BroadcastSender, BroadcastHandler } from './broadcast.js';
export type { PresenceAdapter, PresenceState, PresenceJoinEvent, PresenceLeaveEvent } from './presence.js';
export type { PostgresChangesAdapter, PostgresEvent, PostgresInsert, PostgresUpdate, PostgresDelete } from './postgres.js';

export { createBroadcast } from './broadcast.js';
export { createPresence } from './presence.js';
export { createPostgresChanges } from './postgres.js';
