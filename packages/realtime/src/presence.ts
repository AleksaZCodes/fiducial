/**
 * Contract 2: Presence
 *
 * Shared state about who is online on a channel.  Supabase Realtime tracks
 * join and leave events and provides a current snapshot of every present
 * client's state.
 *
 * The declared shape is `PresenceState`: the per-client payload type.
 * Every accessor and handler is typed against it.
 */

/** A snapshot of all currently present clients, keyed by presence key. */
export type PresenceState<T> = Map<string, T>;

/** Emitted when a client joins the channel. */
export interface PresenceJoinEvent<T> {
  key: string;
  newPresences: T[];
}

/** Emitted when a client leaves the channel. */
export interface PresenceLeaveEvent<T> {
  key: string;
  leftPresences: T[];
}

/** Adapter required to track and broadcast presence. */
export interface PresenceAdapter<T> {
  /** Broadcast this client's own presence state. */
  track(state: T): void | Promise<void>;

  /** Stop broadcasting this client's presence. */
  untrack(): void | Promise<void>;

  /** Current snapshot of all present clients. */
  state(): PresenceState<T>;

  /** Called when any client joins. */
  onJoin(handler: (event: PresenceJoinEvent<T>) => void): () => void;

  /** Called when any client leaves. */
  onLeave(handler: (event: PresenceLeaveEvent<T>) => void): () => void;
}

/**
 * Create a typed presence client from an adapter.
 *
 * @example
 * ```ts
 * type UserPresence = { userId: string; cursor: { x: number; y: number } };
 * const p = createPresence<UserPresence>(myAdapter);
 * p.track({ userId: 'u1', cursor: { x: 10, y: 20 } });
 * p.onJoin(({ key, newPresences }) => console.log(key, newPresences));
 * ```
 */
export function createPresence<T>(adapter: PresenceAdapter<T>): PresenceAdapter<T> {
  // The adapter is already the right shape; this function is the declaration
  // point — it exists so callers write `createPresence(adapter)` rather than
  // casting, and so the type constraint lives here rather than at the call site.
  return adapter;
}
