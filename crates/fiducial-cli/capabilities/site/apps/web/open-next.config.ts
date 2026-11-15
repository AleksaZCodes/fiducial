import { defineCloudflareConfig } from "@opennextjs/cloudflare";

// The site is the Worker: `pnpm deploy` builds it with OpenNext and ships it.
// `apps/web/wrangler.jsonc` is derived from `[deploy]` in fiducial.toml.
export default defineCloudflareConfig();
