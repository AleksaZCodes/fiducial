import { Hono } from "hono";

/**
 * What the platform's adapters read from the environment.
 *
 * On a Worker these are bindings (`c.env.DB`); on Node they are what
 * `src/node.ts` passes. Declared here once so a route is typed the same on both.
 */
export type Env = {
  Bindings: {
    DB?: D1Database;
  };
};

/**
 * The app: no runtime in it. `src/index.ts` serves it on a Worker and
 * `src/node.ts` on Node, and `src/app.test.ts` calls it directly with no server
 * at all, because a Hono app is just `fetch`.
 */
export const app = new Hono<Env>()
  // The self-hosted target's healthcheck reads this (`deploy = "compose"`).
  .get("/healthz", (c) => c.json({ ok: true }))
  .get("/api/hello", (c) => {
    const name = c.req.query("name") ?? "world";
    return c.json({ message: `Hello, ${name}` });
  });

/**
 * The route table as a type. A front end builds its client from it, so a route's
 * request and response are checked at the call site and never restated:
 *
 *   import { hc } from "hono/client";
 *   import type { AppType } from "@{{name}}/api";
 *   const api = hc<AppType>("/");
 *   const { message } = await (await api.api.hello.$get({ query: { name: "a" } })).json();
 *
 * Chain every route onto `app` above: a route registered on its own line is not
 * in this type.
 */
export type AppType = typeof app;
