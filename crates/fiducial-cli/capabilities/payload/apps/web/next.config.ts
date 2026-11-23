import { withPayload } from "@payloadcms/next/withPayload";
import type { NextConfig } from "next";
import { redirects, rewrites } from "./src/routing";
import { securityHeaders } from "./src/security.generated";

export { securityHeaders };

const staticExport = process.env.STATIC_EXPORT === "1";

const shared: NextConfig = {
  reactStrictMode: true,
  poweredByHeader: false,
  images: {
    loader: "custom",
    loaderFile: "./src/lib/cloudflare-image-loader.ts",
    remotePatterns: [
      {
        protocol: "https",
        hostname: "**.r2.cloudflarestorage.com",
      },
      {
        protocol: "https",
        hostname: "**.cloudflare.com",
      },
    ],
  },
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

      async redirects() {
        return redirects;
      },

      async rewrites() {
        return { beforeFiles: [], afterFiles: rewrites, fallback: [] };
      },
    };

export default withPayload(config);
