import type { NextConfig } from "next";
import { redirects, rewrites } from "./src/routing";
import { securityHeaders } from "./src/security.generated";

/**
 * Response headers every page carries, derived into `src/security.generated.ts`
 * by `fid derive` from the platform's one table (`[security]` in
 * `fiducial.toml` is the only choice a product makes: `hsts_preload`).
 *
 * Declared there rather than in a Cloudflare dashboard rule, because a header set
 * in a dashboard is a fact that lives outside the repository: it cannot be
 * reviewed, reverted, diffed, or known about by an agent, and it silently does
 * not exist in a second environment. Principle 4. The same table feeds the
 * Astro, React Router and TanStack Start templates and the self-hosted
 * Caddyfile, so the four cannot disagree.
 *
 * Re-exported so a product can extend rather than fork this file — principle 3,
 * no abstraction without an escape hatch:
 *
 *   headers: async () => [{ source: "/:path*", headers: [...securityHeaders, mine] }]
 */
export { securityHeaders };

/**
 * A static export, for the self-hosted target (`[adapters] deploy = "compose"`).
 *
 * `STATIC_EXPORT=1 next build` (`pnpm build:static`) writes plain files to
 * `out/`, which Caddy serves; there is no Next server at runtime. Two things
 * follow from that, and both are deliberate:
 *
 * - **`headers`, `redirects` and `rewrites` do not exist.** Next applies them in
 *   its server, and there is none. The security headers are Caddy's job then,
 *   derived into the Caddyfile (`fid-compose`), and `redirects` from
 *   `src/routing` are not served; a product that needs one declares it in
 *   `[deploy]`.
 * - **`images.unoptimized`.** Next's image optimiser is a server too.
 *
 * `trailingSlash` makes every page `<path>/index.html`, which a static file
 * server finds without a rewrite rule.
 *
 * An environment variable and not a declaration, because it is read at build
 * time by the build and by nothing else; the fact that the product is deployed
 * static is `[deploy] static_dir`, and the build that follows it sets this.
 */
const staticExport = process.env.STATIC_EXPORT === "1";

/** What both builds share. */
const shared: NextConfig = {
  // Strict mode catches common React mistakes early.
  reactStrictMode: true,

  /**
   * Do not announce the framework and version.
   *
   * `X-Powered-By: Next.js` is free reconnaissance: it tells a scanner which
   * CVE list to try. It is not a vulnerability on its own, which is exactly why
   * it survives — nobody files a bug for it.
   */
  poweredByHeader: false,
};

const config: NextConfig = staticExport
  ? {
      ...shared,
      output: "export",
      trailingSlash: true,
      images: { unoptimized: true },
    }
  : {
      ...shared,

      async headers() {
        return [{ source: "/:path*", headers: securityHeaders }];
      },

      // Empty unless a capability routes URLs (see `src/routing.ts`).
      async redirects() {
        return redirects;
      },

      // After the filesystem, so `/_next/*`, `/api/*` and files in `public/` win.
      async rewrites() {
        return { beforeFiles: [], afterFiles: rewrites, fallback: [] };
      },
    };

export default config;
