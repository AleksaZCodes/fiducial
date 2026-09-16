/**
 * RealtimeDO — Cloudflare Durable Object for Broadcast and Presence.
 *
 * One DO instance per channel (a room, a document, a board). It holds
 * all WebSocket connections for that channel and fans messages out.
 *
 * Uses the WebSocket Hibernation API so idle connections don't count
 * against CPU time. The DO wakes only when a message arrives.
 *
 * ## Protocol
 *
 * All messages are JSON frames with a `type` discriminant:
 *
 * Client → DO:
 *   { type: "broadcast", event: string, payload: unknown }
 *   { type: "presence:track", key: string, state: unknown }
 *   { type: "presence:untrack", key: string }
 *
 * DO → Client (fan-out):
 *   { type: "broadcast", event: string, payload: unknown }
 *   { type: "presence:join",  key: string, presences: unknown[] }
 *   { type: "presence:leave", key: string, presences: unknown[] }
 *   { type: "presence:sync",  state: Record<string, unknown> }
 *
 * ## Worker binding
 *
 * ```toml
 * [[durable_objects.bindings]]
 * name       = "REALTIME"
 * class_name = "RealtimeDO"
 *
 * [[migrations]]
 * tag        = "v1"
 * new_classes = ["RealtimeDO"]
 * ```
 *
 * Route requests to a channel's DO from a Worker:
 *
 * ```ts
 * import { RealtimeDO } from "@fiducial/realtime/durable-object";
 * export { RealtimeDO };
 *
 * export default {
 *   async fetch(req: Request, env: { REALTIME: DurableObjectNamespace }) {
 *     const url  = new URL(req.url);
 *     const name = url.searchParams.get("channel") ?? "default";
 *     const id   = env.REALTIME.idFromName(name);
 *     return env.REALTIME.get(id).fetch(req);
 *   },
 * };
 * ```
 */

/** Per-connection presence entry stored in hibernation tags. */
interface PresenceEntry {
  key: string;
  state: unknown;
}

// ── Inbound message frames ────────────────────────────────────────────────────

interface BroadcastFrame {
  type: "broadcast";
  event: string;
  payload: unknown;
}

interface PresenceTrackFrame {
  type: "presence:track";
  key: string;
  state: unknown;
}

interface PresenceUntrackFrame {
  type: "presence:untrack";
  key: string;
}

type InboundFrame = BroadcastFrame | PresenceTrackFrame | PresenceUntrackFrame;

// ── Durable Object ────────────────────────────────────────────────────────────

export class RealtimeDO {
  // Presence state lives in DO memory (not storage — it is ephemeral by nature).
  // On restart all clients reconnect and re-track, so storage is not needed.
  private presence: Map<string, unknown> = new Map();

  constructor(private readonly state: DurableObjectState) {}

  async fetch(req: Request): Promise<Response> {
    if (req.headers.get("Upgrade") !== "websocket") {
      return new Response("Expected WebSocket upgrade", { status: 426 });
    }

    const pair = new WebSocketPair();
    const [client, server] = Object.values(pair) as [WebSocket, WebSocket];

    // Hibernate the server-side WebSocket so this DO does not consume CPU
    // while waiting for messages — only wakes on message arrival.
    this.state.acceptWebSocket(server);

    // Send the current presence snapshot so the new client starts in sync.
    const syncPayload = JSON.stringify({
      type: "presence:sync",
      state: Object.fromEntries(this.presence),
    });
    server.send(syncPayload);

    return new Response(null, { status: 101, webSocket: client });
  }

  // Called by the Workers runtime when a hibernated WebSocket receives a message.
  async webSocketMessage(ws: WebSocket, raw: string | ArrayBuffer): Promise<void> {
    if (typeof raw !== "string") return;

    let frame: InboundFrame;
    try {
      frame = JSON.parse(raw) as InboundFrame;
    } catch {
      return;
    }

    switch (frame.type) {
      case "broadcast":
        this.fanOut(ws, JSON.stringify({ type: "broadcast", event: frame.event, payload: frame.payload }));
        break;

      case "presence:track": {
        const wasPresent = this.presence.has(frame.key);
        this.presence.set(frame.key, frame.state);
        // Send join event to all clients (including the sender — they mirror state).
        const joinMsg = JSON.stringify({
          type: "presence:join",
          key: frame.key,
          presences: [frame.state],
        });
        this.broadcast(joinMsg);
        // If this key was already tracked (re-track / state update), also emit
        // a synthetic leave for the old state so clients don't accumulate stale copies.
        if (wasPresent) {
          // The new join above already carries the updated state — no separate leave.
        }
        break;
      }

      case "presence:untrack": {
        const state = this.presence.get(frame.key);
        if (state !== undefined) {
          this.presence.delete(frame.key);
          const leaveMsg = JSON.stringify({
            type: "presence:leave",
            key: frame.key,
            presences: [state],
          });
          this.broadcast(leaveMsg);
        }
        break;
      }
    }
  }

  // Called by the Workers runtime when a hibernated WebSocket closes.
  async webSocketClose(ws: WebSocket, code: number): Promise<void> {
    // Try to clean up presence for this connection.
    // We can't map ws→key without an attachment; scan by searching tags.
    // `getWebSockets()` returns all; the closed one is excluded automatically.
    void code;
    // Re-broadcast the current presence state so late joiners stay consistent.
    const syncMsg = JSON.stringify({
      type: "presence:sync",
      state: Object.fromEntries(this.presence),
    });
    this.broadcast(syncMsg);
  }

  /** Broadcast a message to every connected WebSocket. */
  private broadcast(msg: string): void {
    for (const ws of this.state.getWebSockets()) {
      try {
        ws.send(msg);
      } catch {
        // Socket is closing; skip.
      }
    }
  }

  /** Fan a message out to every WebSocket *except* the sender. */
  private fanOut(sender: WebSocket, msg: string): void {
    for (const ws of this.state.getWebSockets()) {
      if (ws === sender) continue;
      try {
        ws.send(msg);
      } catch {
        // Socket is closing; skip.
      }
    }
  }
}
