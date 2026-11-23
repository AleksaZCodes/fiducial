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
import {
  adminLocale,
  defaultLocale,
  entityName,
  payloadLocales,
} from "./src/generated/payload";

export default buildConfig({
  admin: {
    user: Users.slug,
    meta: {
      titleSuffix: `— ${entityName}`,
    },
    ...(adminLocale !== "en" ? { i18n: { defaultLanguage: adminLocale } } : {}), // fid: allow-locale
  },

  collections: [Users, Pages, Media, Products, Variants, Orders, Quotes, Carts, Discounts],

  globals: [SiteSettings],

  editor: lexicalEditor(),

  secret: process.env.PAYLOAD_SECRET ?? "",

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
