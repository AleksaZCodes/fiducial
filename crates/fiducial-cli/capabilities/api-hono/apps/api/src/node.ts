// The Node entry, for the self-hosted target: the same app, served on a port.
//
// `deploy = "compose"` proxies to the port in `[deploy] proxy_port` (default
// 8080) and reads `/healthz`. Environment values the Worker gets as bindings
// are read from `process.env` here.
import { serve } from "@hono/node-server";
import { app } from "./app";

const port = Number(process.env.PORT ?? 8080);

serve({ fetch: app.fetch, port }, (info) => {
  console.log(`api listening on :${info.port}`);
});
