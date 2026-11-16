import handler, { createServerEntry } from "@tanstack/react-start/server-entry";
import { responseHeaders } from "./security.generated";

// Start's own handler, with the security headers set on every response.
export default createServerEntry({
  async fetch(request, opts) {
    const response = await handler.fetch(request, opts);
    // A handler's response can be immutable (a redirect, a cached one), so the
    // headers go on a copy.
    const out = new Response(response.body, response);
    for (const [name, value] of responseHeaders) out.headers.set(name, value);
    return out;
  },
});
