//! Pure rendering: `[brand]` in, static text out.
//!
//! The `fid-brand` executor in `commands/derive.rs` owns the I/O — reading
//! `fiducial.toml` and writing outputs. Everything here is a pure function of
//! the declared facts, so it is testable without a scaffolded product, the
//! same split `i18n.rs` uses for catalog comparison and codegen.

/// Up to two initials from a trading name, uppercased.
///
/// `"Acme Robotics"` → `"AR"`, `"acme"` → `"A"`. A name with no alphabetic
/// character at all — every field here is already validated non-empty by
/// `Brand::validate`, but a name of pure punctuation is not impossible —
/// falls back to `"?"` rather than emitting an empty `<text>` a reader would
/// see as a blank favicon.
pub fn initials(trading_name: &str) -> String {
    let letters: String = trading_name
        .split_whitespace()
        .filter_map(|word| word.chars().find(|c| c.is_alphanumeric()))
        .take(2)
        .flat_map(|c| c.to_uppercase())
        .collect();
    if letters.is_empty() {
        "?".to_string()
    } else {
        letters
    }
}

/// A minimal, scalable favicon: a rounded square in the primary colour with
/// the trading name's initials in the background colour.
///
/// SVG rather than a rasterized format — every modern browser accepts an SVG
/// favicon, and it costs no image-rendering dependency to generate one, since
/// it is text the whole way down. A PNG/ICO fallback is real future work, not
/// something this function pretends to do.
pub fn render_favicon_svg(
    trading_name: &str,
    primary_color: &str,
    background_color: &str,
) -> String {
    let mark = initials(trading_name);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\">\n  \
         <rect width=\"64\" height=\"64\" rx=\"14\" fill=\"{primary_color}\"/>\n  \
         <text x=\"32\" y=\"43\" text-anchor=\"middle\" \
         font-family=\"system-ui, -apple-system, sans-serif\" font-size=\"28\" \
         font-weight=\"700\" fill=\"{background_color}\">{mark}</text>\n\
         </svg>\n"
    )
}

/// `site.webmanifest` pointing at the generated favicon.
///
/// `short_name` is what a phone prints under a home-screen icon, in roughly
/// twelve characters. It used to be a second copy of `trading_name`, which
/// makes the field do nothing — the launcher truncates and the product gets
/// "Fire Outrea…". `[brand] short_name` overrides it; falling back to the
/// trading name is right for the many products whose name is already short.
pub fn render_manifest(
    trading_name: &str,
    short_name: &str,
    primary_color: &str,
    background_color: &str,
) -> String {
    format!(
        "{{\n  \
         \"name\": \"{trading_name}\",\n  \
         \"short_name\": \"{short_name}\",\n  \
         \"icons\": [\n    \
         {{ \"src\": \"/favicon.svg\", \"sizes\": \"any\", \"type\": \"image/svg+xml\" }}\n  \
         ],\n  \
         \"theme_color\": \"{primary_color}\",\n  \
         \"background_color\": \"{background_color}\",\n  \
         \"display\": \"standalone\"\n\
         }}\n"
    )
}

/// `robots.txt` allowing everything and pointing at the sitemap.
pub fn render_robots(domain: &str) -> String {
    format!("User-agent: *\nAllow: /\n\nSitemap: https://{domain}/sitemap.xml\n")
}

/// `sitemap.xml` with a single entry for the domain root.
///
/// A product's real route list is a fact this pipeline is not given — that is
/// a router's declaration, not the brand's — so this seeds the one URL every
/// site has and is deliberately easy to extend by hand until routes become
/// their own declaration.
pub fn render_sitemap(domain: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  \
         <url>\n    <loc>https://{domain}/</loc>\n  </url>\n\
         </urlset>\n"
    )
}

/// A `schema.org` `Organization` record, as JSON-LD.
///
/// Written as a plain `.jsonld` file rather than inlined into a
/// `<script type=\"application/ld+json\">` tag — templating that tag is the
/// web framework's job, and differs between Next.js and SvelteKit. This is
/// the declaration's derivation; wiring it into a page is one `<script>` tag
/// reading this file.
pub fn render_jsonld(
    legal_name: &str,
    trading_name: &str,
    domain: &str,
    contact_email: &str,
) -> String {
    format!(
        "{{\n  \
         \"@context\": \"https://schema.org\",\n  \
         \"@type\": \"Organization\",\n  \
         \"name\": \"{legal_name}\",\n  \
         \"alternateName\": \"{trading_name}\",\n  \
         \"url\": \"https://{domain}\",\n  \
         \"email\": \"{contact_email}\"\n\
         }}\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_take_the_first_letter_of_up_to_two_words() {
        assert_eq!(initials("Acme Robotics"), "AR");
        assert_eq!(initials("acme"), "A");
        assert_eq!(initials("Acme Robotics Corp"), "AR");
    }

    #[test]
    fn initials_never_produce_an_empty_mark() {
        assert_eq!(initials("   "), "?");
        assert_eq!(initials("!!!"), "?");
    }

    #[test]
    fn favicon_embeds_the_initials_and_both_colours() {
        let svg = render_favicon_svg("Acme Robotics", "#0EA5E9", "#0B1120");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("AR"));
        assert!(svg.contains("#0EA5E9"));
        assert!(svg.contains("#0B1120"));
    }

    #[test]
    fn manifest_is_valid_json_pointing_at_the_favicon() {
        let manifest = render_manifest("Acme", "Acme", "#0EA5E9", "#0B1120");
        let value: serde_json::Value = serde_json::from_str(&manifest).expect("valid JSON");
        assert_eq!(value["name"], "Acme");
        assert_eq!(value["icons"][0]["src"], "/favicon.svg");
    }

    #[test]
    fn short_name_is_its_own_fact_not_a_copy_of_the_long_one() {
        // `short_name` is what a launcher prints under an icon, in about twelve
        // characters. It used to be a second copy of the trading name, which
        // makes the field do nothing but truncate.
        let manifest = render_manifest("Fire Outreach Network", "FON", "#b85207", "#110a07");
        let value: serde_json::Value = serde_json::from_str(&manifest).expect("valid JSON");
        assert_eq!(value["name"], "Fire Outreach Network");
        assert_eq!(value["short_name"], "FON");
    }

    #[test]
    fn robots_points_at_the_declared_domains_sitemap() {
        let robots = render_robots("example.com");
        assert!(robots.contains("Sitemap: https://example.com/sitemap.xml"));
    }

    #[test]
    fn sitemap_is_well_formed_xml_for_the_domain_root() {
        let sitemap = render_sitemap("example.com");
        assert!(sitemap.starts_with("<?xml"));
        assert!(sitemap.contains("<loc>https://example.com/</loc>"));
    }

    #[test]
    fn jsonld_is_valid_json_with_the_organization_type() {
        let jsonld = render_jsonld("Example LLC", "Example", "example.com", "hello@example.com");
        let value: serde_json::Value = serde_json::from_str(&jsonld).expect("valid JSON");
        assert_eq!(value["@type"], "Organization");
        assert_eq!(value["name"], "Example LLC");
        assert_eq!(value["email"], "hello@example.com");
    }
}
