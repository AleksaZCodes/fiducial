import { createRequestHandler } from "react-router";
import { responseHeaders } from "../app/security.generated";

const requestHandler = createRequestHandler(
  () => import("virtual:react-router/server-build"),
  import.meta.env.MODE,
);

export default {
  async fetch(request) {
    const response = await requestHandler(request);
    // A handler's response can be immutable (a redirect, a cached one), so the
    // headers go on a copy.
    const out = new Response(response.body, response);
    for (const [name, value] of responseHeaders) out.headers.set(name, value);
    return out;
  },
} satisfies ExportedHandler<Env>;
