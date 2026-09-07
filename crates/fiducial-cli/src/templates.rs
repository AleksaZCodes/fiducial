//! Central registry of all embedded template content.
//!
//! Every template baked into the `fid` binary is accessible here by its
//! repo-relative path (forward slashes). `fid upgrade` uses this to detect
//! upstream template changes and produce 3-way merges.
//!
//! Templates are returned **unexpanded** — placeholders (`{{name}}`,
//! `{{version}}`) are not substituted. The caller must expand before comparing
//! against on-disk content (which was expanded at scaffold time).

// ── `fid new` templates ───────────────────────────────────────────────────────

const TMPL_FIDUCIAL_TOML: &str = include_str!("../templates/fiducial.toml.tmpl");
const TMPL_MISSION_MD: &str = include_str!("../templates/MISSION.md.tmpl");
const TMPL_AGENTS_MD: &str = include_str!("../templates/AGENTS.md.tmpl");
const TMPL_GITIGNORE: &str = include_str!("../templates/gitignore.tmpl");
const TMPL_CLAUDE_SETTINGS: &str = include_str!("../templates/claude-settings.json.tmpl");
const TMPL_AGENT_REVIEW: &str = include_str!("../templates/agents-review.md.tmpl");
const TMPL_AGENT_DESIGN: &str = include_str!("../templates/agents-design.md.tmpl");
const TMPL_CI_REVIEW: &str = include_str!("../templates/claude-review.yml.tmpl");
const TMPL_README: &str = include_str!("../templates/README.md.tmpl");

// ── Capability templates ──────────────────────────────────────────────────────

const CAP_WEB_NEXT_PKG: &str = include_str!("../capabilities/web-next/apps/web/package.json");
const CAP_WEB_NEXT_CONFIG: &str = include_str!("../capabilities/web-next/apps/web/next.config.ts");
const CAP_WEB_NEXT_TSCONFIG: &str = include_str!("../capabilities/web-next/apps/web/tsconfig.json");
const CAP_WEB_NEXT_PAGE: &str = include_str!("../capabilities/web-next/apps/web/src/app/page.tsx");
const CAP_WEB_NEXT_LAYOUT: &str =
    include_str!("../capabilities/web-next/apps/web/src/app/layout.tsx");

const CAP_FIRMWARE_README: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/rp2040/README.md");

const CAP_TAURI_CONF: &str =
    include_str!("../capabilities/tauri/apps/desktop/src-tauri/tauri.conf.json");

const CAP_WORKER_WRANGLER: &str =
    include_str!("../capabilities/worker-cloudflare/apps/worker/wrangler.toml");

// ── Lookup ────────────────────────────────────────────────────────────────────

/// Look up the raw (unexpanded) template for a repo-relative path.
///
/// Returns `None` for paths that are not tracked as templates.
pub fn raw(rel_path: &str) -> Option<&'static str> {
    match rel_path {
        // fid new templates
        "fiducial.toml" => Some(TMPL_FIDUCIAL_TOML),
        "MISSION.md" => Some(TMPL_MISSION_MD),
        "AGENTS.md" => Some(TMPL_AGENTS_MD),
        ".gitignore" => Some(TMPL_GITIGNORE),
        ".claude/settings.json" => Some(TMPL_CLAUDE_SETTINGS),
        ".claude/agents/review.md" => Some(TMPL_AGENT_REVIEW),
        ".claude/agents/design.md" => Some(TMPL_AGENT_DESIGN),
        ".github/workflows/claude-review.yml" => Some(TMPL_CI_REVIEW),
        "README.md" => Some(TMPL_README),
        // web-next capability
        "apps/web/package.json" => Some(CAP_WEB_NEXT_PKG),
        "apps/web/next.config.ts" => Some(CAP_WEB_NEXT_CONFIG),
        "apps/web/tsconfig.json" => Some(CAP_WEB_NEXT_TSCONFIG),
        "apps/web/src/app/page.tsx" => Some(CAP_WEB_NEXT_PAGE),
        "apps/web/src/app/layout.tsx" => Some(CAP_WEB_NEXT_LAYOUT),
        // firmware-rp2040 capability
        "firmware/rp2040/README.md" => Some(CAP_FIRMWARE_README),
        // tauri capability
        "apps/desktop/src-tauri/tauri.conf.json" => Some(CAP_TAURI_CONF),
        // worker-cloudflare capability
        "apps/worker/wrangler.toml" => Some(CAP_WORKER_WRANGLER),
        _ => None,
    }
}

/// Expand `{{name}}` and `{{version}}` placeholders in a raw template.
pub fn expand(raw_template: &str, product_name: &str, platform_version: &str) -> String {
    raw_template
        .replace("{{name}}", product_name)
        .replace("{{version}}", platform_version)
}
