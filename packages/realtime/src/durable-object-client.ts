/**
 * Client-side adapters for RealtimeDO.
 *
 * `connectChannel(url)` opens a WebSocket to a RealtimeDO instance and
 * returns a `{ broadcast, presence }` pair — both typed, both backed by
 * the same connection. The adapters satisfy `BroadcastAdapter<E>` and
 * `PresenceAdapter<T>` so they plug straight into `createBroadcast` /
 * `createPresence` without a shim.
 *
 * @example
 * ```ts
 * import { connectChannel } from "@fiducial/realtime/client";
 * import { createBroadcast, createPresence } from "@fiducial/realtime";
 *
 * type CursorEvents = { cursor: { x: number; y: number } };
 * type PlayerState  = { name: string; ready: boolean };
 *
 * const ch = connectChannel(`wss://my-worker.example.com/channel?channel=room-1`);
 *
 * const broadcast = createBroadcast<CursorEvents>(ch.broadcastAdapter<CursorEvents>());
 * const presence  = createPresence<PlayerState>(ch.presenceAdapter<PlayerState>("alice"));
 *
 * // send / on / track / onJoin / onLeave …
 * ```
 *
 * ## Connection lifecycle
 *
 * `connectChannel` opens the WebSocket immediately. Call `ch.close()` to
 * disconnect. Re-connect by calling `connectChannel` again — the DO state
 * is ephemeral, so the new connection receives a `presence:sync` frame with
 * the current snapshot.
 */

import type { BroadcastAdapter, BroadcastHandler } from "./broadcast.js";
import type { PresenceAdapter, PresenceState, PresenceJoinEvent, PresenceLeaveEvent } from "./presence.js";

// ── Outbound message frames (client → DO) ─────────────────────────────────────

function broadcastFrame(event: string, payload: unknown): string {
  return JSON.stringify({ type: "broadcast", event, payload });
}

function presenceTrackFrame(key: string, state: unknown): string {
  return JSON.stringify({ type: "presence:track", key, state });
}

function presenceUntrackFrame(key: string): string {
  return JSON.stringify({ type: "presence:untrack", key });
}

// ── Channel connection ────────────────────────────────────────────────────────

type AnyHandler = (payload: unknown) => void;

export interface Channel {
  /** Returns a broadcast adapter typed to event map `E`. */
  broadcastAdapter<E extends Record<string, unknown>>(): BroadcastAdapter<E>;

  /** Returns a presence adapter typed to `T`, identified by `key`. */
  presenceAdapter<T>(key: string): PresenceAdapter<T>;

  /** Close the WebSocket connection. */
  close(code?: number, reason?: string): void;

  /** Underlying WebSocket, for advanced use. */
  readonly ws: WebSocket;
}

/**
 * Open a WebSocket connection to a RealtimeDO channel.
 *
 * `url` should point to a Worker route that proxies to the DO, e.g.
 * `wss://worker.example.com/channel?channel=room-42`.
 */
export function connectChannel(url: string | URL): Channel {
  const ws = new WebSocket(url);

  // Registered broadcast handlers, keyed by event name.
  const broadcastHandlers = new Map<string, Set<AnyHandler>>();

  // Presence state mirror (updated by presence:sync / presence:join / presence:leave frames).
  const presenceState = new Map<string, unknown>();

  // Presence join/leave handlers.
  const joinHandlers = new Set<(e: PresenceJoinEvent<unknown>) => void>();
  const leaveHandlers = new Set<(e: PresenceLeaveEvent<unknown>) => void>();

  ws.addEventListener("message", (ev) => {
    let frame: Record<string, unknown>;
    try {
      frame = JSON.parse(typeof ev.data === "string" ? ev.data : "") as Record<string, unknown>;
    } catch {
      return;
    }

    switch (frame.type) {
      case "broadcast": {
        const handlers = broadcastHandlers.get(frame.event as string);
        if (handlers) {
          for (const h of handlers) h(frame.payload);
        }
        break;
      }

      case "presence:sync": {
        presenceState.clear();
        const state = frame.state as Record<string, unknown>;
        for (const [k, v] of Object.entries(state)) {
          presenceState.set(k, v);
        }
        break;
      }

      case "presence:join": {
        const key = frame.key as string;
        const presences = frame.presences as unknown[];
        // Update local state with the latest entry from this key.
        if (presences.length > 0) presenceState.set(key, presences[presences.length - 1]);
        for (const h of joinHandlers) h({ key, newPresences: presences });
        break;
      }

      case "presence:leave": {
        const key = frame.key as string;
        const presences = frame.presences as unknown[];
        presenceState.delete(key);
        for (const h of leaveHandlers) h({ key, leftPresences: presences });
        break;
      }
    }
  });

  function sendWhenOpen(msg: string): void {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(msg);
    } else {
      // Queue until open — the DO accepts messages only after handshake.
      ws.addEventListener("open", () => ws.send(msg), { once: true });
    }
  }

  return {
    ws,

    broadcastAdapter<E extends Record<string, unknown>>(): BroadcastAdapter<E> {
      return {
        send<Ev extends keyof E & string>(event: Ev, payload: E[Ev]) {
          sendWhenOpen(broadcastFrame(event, payload));
        },
        on<Ev extends keyof E & string>(event: Ev, handler: BroadcastHandler<E[Ev]>) {
          if (!broadcastHandlers.has(event)) broadcastHandlers.set(event, new Set());
          broadcastHandlers.get(event)!.add(handler as AnyHandler);
          return () => broadcastHandlers.get(event)?.delete(handler as AnyHandler);
        },
      };
    },

    presenceAdapter<T>(key: string): PresenceAdapter<T> {
      return {
        track(state: T) {
          sendWhenOpen(presenceTrackFrame(key, state));
        },
        untrack() {
          sendWhenOpen(presenceUntrackFrame(key));
        },
        state(): PresenceState<T> {
          return new Map(presenceState as Map<string, T>);
        },
        onJoin(handler: (e: PresenceJoinEvent<T>) => void) {
          joinHandlers.add(handler as (e: PresenceJoinEvent<unknown>) => void);
          return () => joinHandlers.delete(handler as (e: PresenceJoinEvent<unknown>) => void);
        },
        onLeave(handler: (e: PresenceLeaveEvent<T>) => void) {
          leaveHandlers.add(handler as (e: PresenceLeaveEvent<unknown>) => void);
          return () => leaveHandlers.delete(handler as (e: PresenceLeaveEvent<unknown>) => void);
        },
      };
    },

    close(code = 1000, reason = "closed") {
      ws.close(code, reason);
    },
  };
}
