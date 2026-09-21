#!/usr/bin/env node
/**
 * Standalone relay process. Run directly:
 *   PORT=8787 node relay-bin.js
 *   PORT=8787 bun relay-bin.ts
 *   npx @fiducial/realtime  (via "realtime-relay" bin entry)
 */

import { createRelay } from "./relay.js";

const port = Number(process.env.PORT ?? 8787);
const host = process.env.HOST ?? "0.0.0.0";

const relay = createRelay({ port, host });
console.log(`@fiducial/realtime relay  ws://${host}:${relay.port}?channel=<name>`);

function shutdown() {
  relay.close();
  process.exit(0);
}

process.on("SIGTERM", shutdown);
process.on("SIGINT", shutdown);
