// The Cloudflare Worker entry: a Hono app is already `{ fetch }`.
//
// `[deploy] shape = "worker"` points the derived `wrangler.toml` here
// (`main = "src/index.ts"`), so there is no entry to keep in step by hand.
import { app } from "./app";

export default app;
