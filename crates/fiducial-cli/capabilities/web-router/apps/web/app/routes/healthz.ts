import type { Route } from "./+types/healthz";

// A resource route: a loader and no component, so the response is whatever it
// returns. It is the JSON endpoint of a full-stack app, and the healthcheck the
// self-hosted target reads (`deploy = "compose"`).
export function loader(_: Route.LoaderArgs) {
  return Response.json({ ok: true });
}
