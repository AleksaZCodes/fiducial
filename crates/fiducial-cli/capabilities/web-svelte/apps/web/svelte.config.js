import adapter from "@sveltejs/adapter-auto";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  kit: {
    adapter: adapter(),

    // The Content-Security-Policy lives here, not in `hooks.server.ts`.
    //
    // SvelteKit starts the client with an inline `<script>` in the body. A
    // hand-written `script-src 'self'` header does not know about that
    // script, so the browser blocks it and the app never hydrates — the
    // server-rendered page still looks right, and every interactive component
    // is dead. `mode: "hash"` makes SvelteKit emit the sha256 of each inline
    // script it generates into these directives, so the allowance is derived
    // from the actual bytes instead of maintained by hand.
    //
    // The rest of the required headers are in `src/hooks.server.ts`, which is
    // also where `fid doctor` looks first; it reads this file too, precisely
    // so CSP can live where it works.
    csp: {
      mode: "hash",
      directives: {
        "default-src": ["self"],
        "script-src": ["self"],
        "style-src": ["self", "unsafe-inline"],
        "img-src": ["self", "data:"],
        "connect-src": ["self"],
        "base-uri": ["none"],
        "form-action": ["self"],
        "frame-ancestors": ["none"],
      },
    },
  },
};

export default config;
