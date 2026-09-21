/**
 * @fiducial/realtime — typed Broadcast and Presence over Cloudflare Durable Objects.
 *
 * Two contracts, one declaration each:
 *
 * 1. **Broadcast** — fire-and-forget ephemeral messages to all subscribers on a
 *    channel.  No persistence, no history.  Backed by a RealtimeDO Durable Object
 *    that fans messages out over WebSocket Hibernation — idle connections do not
 *    consume CPU time.
 *
 * 2. **Presence** — shared state about who is online.  Tracks join/leave events
 *    and exposes a typed snapshot of the current presence set.  State lives in DO
 *    memory; on reconnect the DO sends a `presence:sync` frame with the current
 *    snapshot.
 *
 * Each contract is a thin typed layer over an adapter interface — the package
 * does not import any Cloudflare runtime directly, so tests run without a DO
 * and any WebSocket-backed implementation satisfies the interface.
 *
 * ## Entry points
 *
 * | Import | Contents |
 * |--------|----------|
 * | `@fiducial/realtime` | Contract types + `createBroadcast` / `createPresence` |
 * | `@fiducial/realtime/durable-object` | `RealtimeDO` — the Worker-side DO class |
 * | `@fiducial/realtime/client` | `connectChannel` — client-side WebSocket adapter |
 *
 * ## Postgres Changes
 *
 * CDC from Postgres is Supabase-specific (not a DO contract) and is re-exported
 * from this entry point for convenience — it is an adapter interface like the
 * others, backed by a Supabase channel in practice.  It lives here because it
 * is a realtime pattern, not because DOs implement it.
 */

export type { BroadcastAdapter, BroadcastSender, BroadcastHandler } from './broadcast.js';
export type { PresenceAdapter, PresenceState, PresenceJoinEvent, PresenceLeaveEvent } from './presence.js';
export type { PostgresChangesAdapter, PostgresEvent, PostgresInsert, PostgresUpdate, PostgresDelete } from './postgres.js';

export { createBroadcast } from './broadcast.js';
export { createPresence } from './presence.js';
export { createPostgresChanges } from './postgres.js';
