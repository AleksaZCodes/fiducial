/**
 * Local WebSocket relay — same frame protocol as RealtimeDO, runs anywhere
 * Node ≥ 20 or Bun runs.
 *
 * Use this when:
 * - You need LAN-only or offline communication (no Cloudflare reachability).
 * - You are running tests against the realtime protocol without a DO.
 * - You want a self-hosted relay on bare metal, a Raspberry Pi, or a container.
 *
 * The URL scheme is identical to the DO Worker route:
 *   ws://host:port/channel?channel=<name>
 *
 * `connectChannel()` from `@fiducial/realtime/client` works against this relay
 * without modification — the same adapter interfaces, the same JSON frames.
 *
 * @example
 * ```ts
 * import { createRelay } from "@fiducial/realtime/relay";
 *
 * const relay = createRelay({ port: 8787 });
 * // → ws://0.0.0.0:8787?channel=room-42
 *
 * relay.close(); // graceful shutdown
 * ```
 *
 * Or run as a standalone process:
 * ```sh
 * PORT=8787 npx realtime-relay
 * # or: bun run realtime-relay
 * ```
 */

import { createServer } from "node:http";
import type { IncomingMessage } from "node:http";
import { WebSocketServer, WebSocket } from "ws";

// ── Per-channel state ─────────────────────────────────────────────────────────

interface ChannelState {
  clients: Set<WebSocket>;
  presence: Map<string, unknown>;
}

// ── Public API ────────────────────────────────────────────────────────────────

export interface RelayOptions {
  /** Port to listen on. Defaults to 8787. */
  port?: number;
  /** Host / bind address. Defaults to "0.0.0.0". */
  host?: string;
}

export interface Relay {
  /** Actual port the server is listening on (useful when port 0 was passed). */
  readonly port: number;
  /** Gracefully close all connections and stop the server. */
  close(): void;
}

/**
 * Start a local relay server that speaks the RealtimeDO frame protocol.
 *
 * Multiple channels are multiplexed over a single port — the channel name
 * comes from the `?channel=` query parameter, defaulting to `"default"`.
 */
export function createRelay(options: RelayOptions = {}): Relay {
  const { port = 8787, host = "0.0.0.0" } = options;

  // Channel registry — entries are created on first join and removed when empty.
  const channels = new Map<string, ChannelState>();

  function getChannel(name: string): ChannelState {
    let ch = channels.get(name);
    if (!ch) {
      ch = { clients: new Set(), presence: new Map() };
      channels.set(name, ch);
    }
    return ch;
  }

  // Plain HTTP server — only WebSocket upgrades are meaningful.
  const server = createServer((_req, res) => {
    res.writeHead(426, { "Content-Type": "text/plain" });
    res.end("Expected WebSocket upgrade");
  });

  const wss = new WebSocketServer({ server });

  wss.on("connection", (ws: WebSocket, req: IncomingMessage) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    const channelName = url.searchParams.get("channel") ?? "default";
    const ch = getChannel(channelName);

    ch.clients.add(ws);

    // Sync the new client to current presence state.
    ws.send(
      JSON.stringify({ type: "presence:sync", state: Object.fromEntries(ch.presence) }),
    );

    ws.on("message", (raw) => {
      const str =
        typeof raw === "string" ? raw : Buffer.isBuffer(raw) ? raw.toString("utf8") : null;
      if (!str) return;

      let frame: Record<string, unknown>;
      try {
        frame = JSON.parse(str) as Record<string, unknown>;
      } catch {
        return;
      }

      switch (frame.type) {
        case "broadcast":
          fanOut(
            ch,
            ws,
            JSON.stringify({ type: "broadcast", event: frame.event, payload: frame.payload }),
          );
          break;

        case "presence:track": {
          ch.presence.set(frame.key as string, frame.state);
          broadcast(
            ch,
            JSON.stringify({ type: "presence:join", key: frame.key, presences: [frame.state] }),
          );
          break;
        }

        case "presence:untrack": {
          const state = ch.presence.get(frame.key as string);
          if (state !== undefined) {
            ch.presence.delete(frame.key as string);
            broadcast(
              ch,
              JSON.stringify({ type: "presence:leave", key: frame.key, presences: [state] }),
            );
          }
          break;
        }
      }
    });

    ws.on("close", () => {
      ch.clients.delete(ws);
      // Re-broadcast sync so remaining clients converge on consistent state.
      broadcast(
        ch,
        JSON.stringify({ type: "presence:sync", state: Object.fromEntries(ch.presence) }),
      );
      if (ch.clients.size === 0) channels.delete(channelName);
    });
  });

  server.listen(port, host);

  return {
    get port(): number {
      const addr = server.address();
      return typeof addr === "object" && addr !== null ? addr.port : port;
    },
    close(): void {
      wss.close();
      server.closeAllConnections?.();
      server.close();
    },
  };
}

// ── Internal helpers ──────────────────────────────────────────────────────────

function broadcast(ch: ChannelState, msg: string): void {
  for (const ws of ch.clients) {
    if (ws.readyState === WebSocket.OPEN) {
      try {
        ws.send(msg);
      } catch {
        // closing; skip
      }
    }
  }
}

function fanOut(ch: ChannelState, sender: WebSocket, msg: string): void {
  for (const ws of ch.clients) {
    if (ws === sender) continue;
    if (ws.readyState === WebSocket.OPEN) {
      try {
        ws.send(msg);
      } catch {
        // closing; skip
      }
    }
  }
}
