import type { NextConfig } from "next";

// From the public config type, not Next's internals, which move between majors.
type Redirect = Awaited<ReturnType<NonNullable<NextConfig["redirects"]>>>[number];
type Rewrite = Extract<
  Awaited<ReturnType<NonNullable<NextConfig["rewrites"]>>>,
  readonly unknown[]
>[number];

/**
 * The app's URL rules, spread into `next.config.ts`.
 *
 * Empty here: a plain app has one URL per page. A capability that changes how
 * URLs map to pages — `site`, which serves the default locale without a prefix
 * and every other one under `/<locale>` — replaces this file rather than
 * editing `next.config.ts`, so the security headers there stay one copy.
 */
export const redirects: Redirect[] = [];
export const rewrites: Rewrite[] = [];
