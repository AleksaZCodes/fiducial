// A server-only module: the `.server` suffix is React Router's convention, and the
// build refuses to let the browser bundle import it. Put what must never reach a
// client here: secrets, database access, anything that reads a binding.
//
// In a loader or action on the Worker, bindings come from the runtime:
//
//   import { env } from "cloudflare:workers";
//   const rows = await env.DB.prepare("select 1").all();   // typed by `wrangler types`

export function greetingFor(name: string): string {
  return `Hello, ${name}. This was rendered on the Worker.`;
}
