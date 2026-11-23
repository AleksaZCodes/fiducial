//! Analytics providers — registry, validation, and problem reporting.
//!
//! The registry is the single declaration of what each provider is, what it
//! stores, and whether it requires the `consent` capability. Everything that
//! reads `requires_consent` without `node_modules` in sight — `fid doctor`,
//! the config validator, and the generated legal page — reads it here.
//!
//! Spec: `docs/specs/2026-11-23-analytics-is-a-capability-consent-is-opt-in.md`
//! D17 · D22.

/// One entry in the analytics provider registry.
pub struct Provider {
    pub id: &'static str,
    /// True when this provider must not load until the visitor has consented.
    /// A product that lists a requires_consent provider without the `consent`
    /// capability fails `fid derive` and is reported by `fid doctor` (D22).
    pub requires_consent: bool,
    /// Whether this provider is included when the capability seeds the config.
    pub default_on: bool,
    /// Secret or environment-variable names this provider reads.
    pub env_vars: &'static [&'static str],
    /// One sentence: what this provider stores about a visitor.
    pub stores: &'static str,
}

/// Every analytics provider the platform knows.
pub static PROVIDERS: &[Provider] = &[
    Provider {
        id: "first-party",
        requires_consent: false,
        default_on: true,
        env_vars: &["ANALYTICS_HMAC_SECRET"],
        stores: "one D1 row per event: visitor_hash (daily-rotating HMAC, no raw IP), \
                 event type, path, utm_source/medium/campaign, referrer host, timestamp",
    },
    Provider {
        id: "cloudflare-web-analytics",
        requires_consent: false,
        default_on: true,
        env_vars: &[],
        stores: "nothing on the device — cookieless beacon processed by Cloudflare",
    },
    Provider {
        id: "posthog",
        requires_consent: false,
        default_on: false,
        env_vars: &["POSTHOG_API_KEY"],
        stores: "nothing on the device; cookieless_mode always; EU host",
    },
    Provider {
        id: "meta-pixel-capi",
        requires_consent: true,
        default_on: false,
        env_vars: &["META_PIXEL_ID", "META_ACCESS_TOKEN"],
        stores: "hashed email and phone via Meta Conversions API — requires consent",
    },
];

/// Look up a provider by id.
pub fn find(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.id == id)
}

/// Return a problem string when any listed provider requires consent and the
/// `consent` capability is not in the enabled list. `None` means no problem.
///
/// Used by both the config validator (which bails) and `fid doctor` (which
/// appends to its issues list without throwing — the same split `adapter::problem`
/// uses so `fid doctor` can describe a problem rather than dying on it).
pub fn problem(providers: &[String], capabilities_enabled: &[String]) -> Option<String> {
    let has_consent = capabilities_enabled.iter().any(|c| c == "consent");
    for id in providers {
        let requires = find(id).map_or(false, |p| p.requires_consent);
        if requires && !has_consent {
            return Some(format!(
                "[analytics] providers includes \"{id}\", which requires the `consent` \
                 capability. Add it (`fid add capability consent`) or remove \"{id}\" \
                 from [analytics] providers."
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_has_a_non_empty_id() {
        for p in PROVIDERS {
            assert!(!p.id.is_empty(), "provider with empty id");
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for p in PROVIDERS {
            assert!(seen.insert(p.id), "duplicate provider id: {}", p.id);
        }
    }

    #[test]
    fn meta_pixel_requires_consent() {
        let p = find("meta-pixel-capi").expect("meta-pixel-capi in registry");
        assert!(p.requires_consent);
    }

    #[test]
    fn first_party_does_not_require_consent() {
        let p = find("first-party").expect("first-party in registry");
        assert!(!p.requires_consent);
    }

    #[test]
    fn cloudflare_web_analytics_does_not_require_consent() {
        let p = find("cloudflare-web-analytics").expect("cloudflare-web-analytics in registry");
        assert!(!p.requires_consent);
    }

    #[test]
    fn no_problem_without_requires_consent_providers() {
        let providers = vec!["first-party".into(), "cloudflare-web-analytics".into()];
        assert!(problem(&providers, &[]).is_none());
    }

    #[test]
    fn problem_when_meta_pixel_without_consent_capability() {
        let providers = vec!["meta-pixel-capi".into()];
        let p = problem(&providers, &[]).expect("should be a problem");
        assert!(p.contains("consent"), "{p}");
        assert!(p.contains("meta-pixel-capi"), "{p}");
    }

    #[test]
    fn no_problem_when_meta_pixel_with_consent_capability() {
        let providers = vec!["meta-pixel-capi".into()];
        let caps = vec!["consent".into()];
        assert!(problem(&providers, &caps).is_none());
    }
}
