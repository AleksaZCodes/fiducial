//! Response headers every web app must send, declared once.
//!
//! # Why this is a module and not a template default
//!
//! A template is a starting point. A product edits `next.config.ts` for its own
//! reasons, drops a header while doing it, and nothing notices — the page still
//! renders, the build still passes, and the gap is visible only to whoever
//! thinks to run `curl -I` against production. That is how
//! `fire.outreachnet.work` came to serve **no security headers at all** on
//! 2026-09-20: nothing was misconfigured, the capability template simply never
//! had them, and no gate existed to say so.
//!
//! So the set lives here, in one place that two things read:
//!
//! - **The capability templates** apply it, each in its framework's idiom — a
//!   `headers()` function in Next, a `handle` hook in SvelteKit, a `fetch`
//!   wrapper in a Worker. One fact, several derivations.
//! - **`fid doctor`** verifies a product still sends it. That is the half that
//!   makes this an *ensure* rather than a default.
//!
//! Both reading the same list is the point: a checker with its own copy of the
//! expected headers is a second declaration, and the two drift the first time
//! one is updated alone.
//!
//! # Why the check reads the repository, not the network
//!
//! `fid` makes no network calls (see `commands/advise.rs` for the same boundary),
//! and a gate that probes production cannot run in CI before a deploy, cannot
//! run offline, and answers about whatever is deployed rather than what is about
//! to be. So this checks the **declaration**. A declaration that is present and
//! wrong is still possible; a declaration that is absent is the failure that
//! actually happened.

/// One header, why it exists, and how to recognise it in a config file.
pub struct SecurityHeader {
    /// Canonical header name, as sent.
    pub name: &'static str,
    /// What it stops. Printed by `fid doctor` when it is missing, because a
    /// finding that does not say what it protects gets waived.
    pub protects: &'static str,
    /// Whether a web app is *required* to send it.
    ///
    /// `Content-Security-Policy` is required; a strict `script-src` is not yet,
    /// because Next.js needs nonce middleware before `'unsafe-inline'` can go.
    /// Recording that as a level rather than omitting the header keeps the debt
    /// visible — see `docs/specs/2026-09-20-hsts-preload.md`.
    pub required: bool,
}

/// The canonical set. Ordered as a reader would want them explained, not
/// alphabetically — `fid doctor` prints them in this order.
pub static SECURITY_HEADERS: &[SecurityHeader] = &[
    SecurityHeader {
        name: "Strict-Transport-Security",
        protects: "a downgrade attack on a returning visitor. Needs max-age >= 1 year \
             and includeSubDomains to be preload-eligible",
        required: true,
    },
    SecurityHeader {
        name: "Content-Security-Policy",
        protects: "injected <base> re-rooting every relative URL, a planted form \
             posting credentials off-site, plugin embedding, and framing",
        required: true,
    },
    SecurityHeader {
        name: "X-Content-Type-Options",
        protects: "an upload served back as text/html because its bytes looked like markup",
        required: true,
    },
    SecurityHeader {
        name: "Referrer-Policy",
        protects: "leaking full paths cross-site, and any referrer over plain HTTP",
        required: true,
    },
    SecurityHeader {
        name: "X-Frame-Options",
        protects: "clickjacking in a browser too old for CSP frame-ancestors, which is \
             why both are sent",
        required: true,
    },
    SecurityHeader {
        name: "Permissions-Policy",
        protects: "capabilities a content site never needs — camera, microphone, \
             geolocation, payment, usb",
        required: true,
    },
    SecurityHeader {
        name: "Cross-Origin-Opener-Policy",
        protects: "a popup reaching back through window.opener",
        required: true,
    },
];

/// Where a framework declares its response headers.
///
/// Each entry is the file `fid doctor` reads for a product that has that app
/// shape. A framework absent from this list is a framework whose headers nothing
/// checks, which is worth being explicit about rather than silently true.
pub struct WebAppShape {
    /// Human name, for the doctor line.
    pub name: &'static str,
    /// Path relative to the product root.
    pub config: &'static str,
    /// Present only when this shape is actually installed.
    pub marker: &'static str,
    /// Other files that may legitimately carry some of the headers.
    ///
    /// A framework sometimes owns a header itself, and owning it is better
    /// than restating it: a header declared where the framework can complete
    /// it beats the same header hand-written where the framework cannot. The
    /// check reads these alongside `config` so doing the correct thing does
    /// not fail the gate — see the SvelteKit entry for the case this exists
    /// for.
    pub also: &'static [&'static str],
}

pub static WEB_APP_SHAPES: &[WebAppShape] = &[
    WebAppShape {
        name: "Next.js",
        config: "apps/web/next.config.ts",
        marker: "apps/web/next.config.ts",
        also: &[],
    },
    WebAppShape {
        name: "SvelteKit",
        config: "apps/web/src/hooks.server.ts",
        marker: "apps/web/svelte.config.js",
        // `svelte.config.js` counts too, and for CSP it is the **right**
        // place rather than an alternative one.
        //
        // SvelteKit boots the client from an inline `<script>` it generates.
        // A `Content-Security-Policy` written by hand in `hooks.server.ts`
        // cannot know that script's hash, so `script-src 'self'` blocks it and
        // the app never hydrates — SSR output still looks perfect, and every
        // interactive component is silently dead. That shipped: a countdown
        // frozen at 00:00:00 and a language picker that would not open, on a
        // product whose build, typecheck and this very check were all green.
        //
        // `kit.csp` in `svelte.config.js` is the declaration SvelteKit can
        // derive the hashes into. Looking here as well is what lets a product
        // do the correct thing without failing the gate for it.
        also: &["apps/web/svelte.config.js"],
    },
    WebAppShape {
        name: "Cloudflare Worker",
        config: "src/index.ts",
        marker: "wrangler.jsonc",
        also: &[],
    },
];

/// The required headers a config file does not mention.
///
/// Matched by name, case-insensitively, anywhere in the file. That is a coarse
/// test on purpose: parsing TypeScript to prove a header reaches a response
/// would be a compiler, and the failure this catches is the header being *absent
/// entirely*, which a name search answers exactly. A product that imports the
/// shared `securityHeaders` constant satisfies it by naming the headers in the
/// file it imports from — so the check also accepts an import of that symbol.
pub fn missing_headers(config_source: &str) -> Vec<&'static SecurityHeader> {
    let haystack = config_source.to_ascii_lowercase();

    // A product extending the platform constant is compliant without naming
    // every header itself — that is the escape hatch working, not a gap.
    if haystack.contains("securityheaders") {
        return Vec::new();
    }

    SECURITY_HEADERS
        .iter()
        .filter(|h| h.required && !haystack.contains(&h.name.to_ascii_lowercase()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_config_with_nothing_is_missing_everything_required() {
        let missing = missing_headers("export default { reactStrictMode: true };");
        assert_eq!(
            missing.len(),
            SECURITY_HEADERS.iter().filter(|h| h.required).count()
        );
    }

    #[test]
    fn importing_the_shared_constant_satisfies_the_check() {
        // The escape hatch: extend rather than restate. A product doing this is
        // compliant, and a check that failed it would push products to copy the
        // list — which is the duplication this module exists to prevent.
        let src = r#"import { securityHeaders } from "./security";"#;
        assert!(missing_headers(src).is_empty());
    }

    #[test]
    fn naming_every_header_inline_also_satisfies_it() {
        let mut src = String::new();
        for h in SECURITY_HEADERS.iter().filter(|h| h.required) {
            src.push_str(h.name);
            src.push('\n');
        }
        assert!(missing_headers(&src).is_empty());
    }

    #[test]
    fn one_absent_header_is_reported_by_name() {
        let mut src = String::new();
        for h in SECURITY_HEADERS.iter().filter(|h| h.required) {
            if h.name == "Referrer-Policy" {
                continue;
            }
            src.push_str(h.name);
            src.push('\n');
        }
        let missing = missing_headers(&src);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].name, "Referrer-Policy");
    }

    #[test]
    fn matching_ignores_case_as_http_does() {
        let mut src = String::new();
        for h in SECURITY_HEADERS.iter().filter(|h| h.required) {
            src.push_str(&h.name.to_ascii_lowercase());
            src.push('\n');
        }
        assert!(missing_headers(&src).is_empty());
    }

    #[test]
    fn every_header_says_what_it_protects() {
        // A finding without a consequence gets waived, so the text is required
        // rather than optional.
        for h in SECURITY_HEADERS {
            assert!(h.protects.len() > 20, "{} has no rationale", h.name);
        }
    }
}
