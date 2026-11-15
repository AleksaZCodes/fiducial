import type { NextConfig } from "next";
import { defaultLocale, locales } from "./generated/messages";

// From the public config type, not Next's internals, which move between majors.
type Redirect = Awaited<ReturnType<NonNullable<NextConfig["redirects"]>>>[number];
type Rewrite = Extract<
  Awaited<ReturnType<NonNullable<NextConfig["rewrites"]>>>,
  readonly unknown[]
>[number];

/**
 * One URL per page per language: the default locale is served bare
 * (`/blog/hello`), every other one under its prefix (`/sr/blog/hello`).
 *
 * Done as config rewrites rather than in a proxy: they run in the router with
 * no code per request, and they work on Workers (OpenNext) as they do on Node.
 */
const others = locales.filter((l) => l !== defaultLocale).join("|");
const bare = others ? `/:path((?!(?:${others})(?:/|$)).+)` : "/:path+";

/** `/en/…` is the same page as `/…`, so it redirects there: one canonical URL. */
export const redirects: Redirect[] = [
  { source: `/${defaultLocale}`, destination: "/", permanent: true },
  { source: `/${defaultLocale}/:path*`, destination: "/:path*", permanent: true },
];

/** Bare paths are the default locale's pages. After the filesystem, so files in `public/` win. */
export const rewrites: Rewrite[] = [
  { source: "/", destination: `/${defaultLocale}` },
  { source: bare, destination: `/${defaultLocale}/:path*` },
];
