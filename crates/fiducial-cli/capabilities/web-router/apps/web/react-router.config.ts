import type { Config } from "@react-router/dev/config";

export default {
  // Server-rendered on the Worker. `false` makes the app a single-page app, but
  // the Cloudflare plugin does not support SPA mode or prerendering, so a
  // static site belongs on `web-astro`, not here.
  ssr: true,
} satisfies Config;
