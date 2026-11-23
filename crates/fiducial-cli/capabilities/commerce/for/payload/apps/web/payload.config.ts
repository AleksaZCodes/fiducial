import { buildConfig } from "payload";
import { sqliteD1Adapter } from "@payloadcms/db-d1-sqlite";
import { r2Storage } from "@payloadcms/storage-r2";
import { lexicalEditor } from "@payloadcms/richtext-lexical";
import { Users } from "./src/payload/collections/Users";
import { Pages } from "./src/payload/collections/Pages";
import { Media } from "./src/payload/collections/Media";
import { Products } from "./src/payload/collections/Products";
import { Variants } from "./src/payload/collections/Variants";
import { Orders } from "./src/payload/collections/Orders";
import { Quotes } from "./src/payload/collections/Quotes";
import { Carts } from "./src/payload/collections/Carts";
import { Discounts } from "./src/payload/collections/Discounts";
import { SiteSettings } from "./src/payload/globals/SiteSettings";
import { getAllMetrics } from "./src/payload/metrics";
import { notifyNewOrderTask, notifyNewQuoteTask } from "./src/payload/jobs";
import { can } from "@fiducial/identity";
import type { Principal, Resource } from "@fiducial/identity";
import { grantsFor, principalFrom } from "./src/payload/access";
import {
  adminLocale,
  defaultLocale,
  entityName,
  payloadLocales,
} from "./src/generated/payload";

type MetricsUser = { id: string; role: "owner" | "staff" | "agency-admin" };

export default buildConfig({
  admin: {
    user: Users.slug,
    meta: {
      titleSuffix: `— ${entityName}`,
    },
    ...(adminLocale !== "en" ? { i18n: { defaultLanguage: adminLocale } } : {}), // fid: allow-locale
    components: {
      views: {
        Dashboard: {
          Component: "./src/payload/views/Dashboard.js#Dashboard",
        },
      },
    },
  },

  endpoints: [
    {
      path: "/commerce-metrics",
      method: "get",
      handler: async (req) => {
        const user = req.user as MetricsUser | null;
        if (!user) return Response.json({ error: "Unauthorized" }, { status: 401 });
        const principal: Principal = principalFrom(user);
        const resource: Resource = { kind: "platform" };
        if (!can(principal, "read", resource, grantsFor(user))) {
          return Response.json({ error: "Forbidden" }, { status: 403 });
        }
        const metrics = await getAllMetrics();
        return Response.json(metrics);
      },
    },
  ],

  collections: [Users, Pages, Media, Products, Variants, Orders, Quotes, Carts, Discounts],

  globals: [SiteSettings],

  jobs: {
    tasks: [notifyNewOrderTask, notifyNewQuoteTask],
  },

  editor: lexicalEditor(),

  secret: process.env.PAYLOAD_SECRET ?? (() => { throw new Error("PAYLOAD_SECRET is required"); })(),

  typescript: {
    outputFile: "./src/payload/payload-types.ts",
  },

  localization: {
    locales: payloadLocales.map(({ code, label }) => ({ code, label })),
    defaultLocale,
    fallback: true,
  },

  db: sqliteD1Adapter({
    // @ts-expect-error env is injected by the Cloudflare Workers runtime
    binding: (process.env as unknown as { DB: D1Database }).DB,
  }),

  plugins: [
    r2Storage({
      collections: {
        media: true,
      },
      // @ts-expect-error env is injected by the Cloudflare Workers runtime
      bucket: (process.env as unknown as { BUCKET: R2Bucket }).BUCKET,
      clientUploads: true,
    }),
  ],
});
