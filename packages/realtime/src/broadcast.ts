/**
 * Contract 1: Broadcast
 *
 * Fire-and-forget ephemeral messages sent to every subscriber on a channel.
 * No persistence, no delivery acknowledgement, no ordering guarantee.
 *
 * The declared shape is a typed event map: `{ [eventName]: payloadType }`.
 * Every send and every handler is checked against it at compile time.
 */

/** Adapter required to send and receive broadcast events. */
export interface BroadcastAdapter<Events extends Record<string, unknown>> {
  /**
   * Send an event to all subscribers (including the sender, unless the
   * Supabase channel is configured with `self: false`).
   */
  send<E extends keyof Events & string>(event: E, payload: Events[E]): void | Promise<void>;

  /**
   * Register a handler for a specific event.  Returns an unsubscribe
   * function.
   */
  on<E extends keyof Events & string>(
    event: E,
    handler: BroadcastHandler<Events[E]>
  ): () => void;
}

/** Typed sender: narrows to a single event. */
export type BroadcastSender<T> = (payload: T) => void | Promise<void>;

/** Typed handler. */
export type BroadcastHandler<T> = (payload: T) => void;

/**
 * Create a typed broadcast client from an adapter.
 *
 * Returns `{ send, on }` with full event-map types.
 *
 * @example
 * ```ts
 * type GameEvents = { score: { team: string; points: number } };
 * const bc = createBroadcast<GameEvents>(myAdapter);
 * bc.send('score', { team: 'red', points: 3 });
 * bc.on('score', ({ team, points }) => console.log(team, points));
 * ```
 */
export function createBroadcast<Events extends Record<string, unknown>>(
  adapter: BroadcastAdapter<Events>
): {
  send: <E extends keyof Events & string>(event: E, payload: Events[E]) => void | Promise<void>;
  on: <E extends keyof Events & string>(event: E, handler: BroadcastHandler<Events[E]>) => () => void;
} {
  return {
    send: (event, payload) => adapter.send(event, payload),
    on: (event, handler) => adapter.on(event, handler),
  };
}
