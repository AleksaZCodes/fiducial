import { sqliteD1Adapter } from "@payloadcms/db-d1-sqlite";
import { lexicalEditor } from "@payloadcms/richtext-lexical";
import { r2Storage } from "@payloadcms/storage-r2";
import { buildConfig } from "payload";
import { adminLocale, defaultLocale, entityName, payloadLocales } from "./src/generated/payload";
import { Media } from "./src/payload/collections/Media";
import { Pages } from "./src/payload/collections/Pages";
import { Users } from "./src/payload/collections/Users";
import { SiteSettings } from "./src/payload/globals/SiteSettings";

export default buildConfig({
  admin: {
    user: Users.slug,
    meta: {
      titleSuffix: `— ${entityName}`,
    },
    ...(adminLocale !== "en" ? { i18n: { defaultLanguage: adminLocale } } : {}), // fid: allow-locale
  },

  collections: [Users, Pages, Media],

  globals: [SiteSettings],

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
