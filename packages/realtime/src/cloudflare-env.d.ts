/**
 * Minimal ambient types for the Cloudflare Workers runtime used by
 * durable-object.ts. These are not part of the standard DOM lib.
 *
 * In a real Worker project these come from @cloudflare/workers-types.
 * We declare just enough here to keep this package typechecking without
 * pulling in the full CF types as a dependency — the DO file is compiled
 * by wrangler in practice, not by this tsconfig.
 */

declare class DurableObjectState {
  acceptWebSocket(ws: WebSocket): void;
  getWebSockets(): WebSocket[];
}

declare class WebSocketPair {
  readonly 0: WebSocket;
  readonly 1: WebSocket;
  [key: number]: WebSocket;
}

// Cloudflare extends ResponseInit with a `webSocket` field for 101 responses.
interface ResponseInit {
  webSocket?: WebSocket;
}
