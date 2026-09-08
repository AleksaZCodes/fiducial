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
const CAP_WEB_NEXT_GLOBALS: &str =
    include_str!("../capabilities/web-next/apps/web/src/app/globals.css");
const CAP_WEB_NEXT_COMPONENTS_JSON: &str =
    include_str!("../capabilities/web-next/apps/web/components.json");

// ── web-svelte capability templates ──────────────────────────────────────────

const CAP_WEB_SVELTE_SVELTE_CONFIG: &str =
    include_str!("../capabilities/web-svelte/apps/web/svelte.config.js");
const CAP_WEB_SVELTE_VITE_CONFIG: &str =
    include_str!("../capabilities/web-svelte/apps/web/vite.config.ts");
const CAP_WEB_SVELTE_APP_HTML: &str =
    include_str!("../capabilities/web-svelte/apps/web/src/app.html");
const CAP_WEB_SVELTE_APP_CSS: &str =
    include_str!("../capabilities/web-svelte/apps/web/src/app.css");
const CAP_WEB_SVELTE_LAYOUT: &str =
    include_str!("../capabilities/web-svelte/apps/web/src/routes/+layout.svelte");
const CAP_WEB_SVELTE_PAGE: &str =
    include_str!("../capabilities/web-svelte/apps/web/src/routes/+page.svelte");

// ── firmware-rp2040 capability templates ─────────────────────────────────────

const CAP_FIRMWARE_RP2040_WORKSPACE: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/Cargo.toml");
const CAP_FIRMWARE_RP2040_TOOLCHAIN: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/rust-toolchain.toml");
const CAP_FIRMWARE_RP2040_SHARED_CARGO: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/shared/Cargo.toml");
const CAP_FIRMWARE_RP2040_SHARED_LIB: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/shared/src/lib.rs");
const CAP_FIRMWARE_RP2040_CARGO_CONFIG: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/rp2040/.cargo/config.toml");
const CAP_FIRMWARE_RP2040_CARGO: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/rp2040/Cargo.toml");
const CAP_FIRMWARE_RP2040_BUILD: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/rp2040/build.rs");
const CAP_FIRMWARE_RP2040_MEMORY: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/rp2040/memory.x");
const CAP_FIRMWARE_RP2040_MAIN: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/rp2040/src/main.rs");
const CAP_FIRMWARE_README: &str =
    include_str!("../capabilities/firmware-rp2040/firmware/rp2040/README.md");

// ── firmware-stm32 capability templates ──────────────────────────────────────

const CAP_FIRMWARE_STM32_WORKSPACE: &str =
    include_str!("../capabilities/firmware-stm32/firmware/Cargo.toml");
const CAP_FIRMWARE_STM32_TOOLCHAIN: &str =
    include_str!("../capabilities/firmware-stm32/firmware/rust-toolchain.toml");
const CAP_FIRMWARE_STM32_SHARED_CARGO: &str =
    include_str!("../capabilities/firmware-stm32/firmware/shared/Cargo.toml");
const CAP_FIRMWARE_STM32_SHARED_LIB: &str =
    include_str!("../capabilities/firmware-stm32/firmware/shared/src/lib.rs");
const CAP_FIRMWARE_STM32_CARGO_CONFIG: &str =
    include_str!("../capabilities/firmware-stm32/firmware/stm32/.cargo/config.toml");
const CAP_FIRMWARE_STM32_CARGO: &str =
    include_str!("../capabilities/firmware-stm32/firmware/stm32/Cargo.toml");
const CAP_FIRMWARE_STM32_BUILD: &str =
    include_str!("../capabilities/firmware-stm32/firmware/stm32/build.rs");
const CAP_FIRMWARE_STM32_MEMORY: &str =
    include_str!("../capabilities/firmware-stm32/firmware/stm32/memory.x");
const CAP_FIRMWARE_STM32_MAIN: &str =
    include_str!("../capabilities/firmware-stm32/firmware/stm32/src/main.rs");

const CAP_TAURI_CONF: &str =
    include_str!("../capabilities/tauri/apps/desktop/src-tauri/tauri.conf.json");
const CAP_TAURI_CARGO: &str =
    include_str!("../capabilities/tauri/apps/desktop/src-tauri/Cargo.toml.tmpl");
const CAP_TAURI_BUILD: &str = include_str!("../capabilities/tauri/apps/desktop/src-tauri/build.rs");
const CAP_TAURI_LIB: &str = include_str!("../capabilities/tauri/apps/desktop/src-tauri/src/lib.rs");
const CAP_TAURI_MAIN: &str =
    include_str!("../capabilities/tauri/apps/desktop/src-tauri/src/main.rs");

const CAP_WORKER_WRANGLER: &str =
    include_str!("../capabilities/worker-cloudflare/apps/worker/wrangler.toml");

// ── eda capability templates ──────────────────────────────────────────────────

const CAP_EDA_MAIN_ATO: &str = include_str!("../capabilities/eda/board/main.ato");
const CAP_EDA_BOARD_INTERFACE: &str =
    include_str!("../capabilities/eda/board/board.interface.json");
const CAP_EDA_PIPELINE: &str = include_str!("../capabilities/eda/pipelines/eda.toml");

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
        "apps/web/src/app/globals.css" => Some(CAP_WEB_NEXT_GLOBALS),
        "apps/web/components.json" => Some(CAP_WEB_NEXT_COMPONENTS_JSON),
        // web-svelte capability (paths not shared with web-next)
        "apps/web/svelte.config.js" => Some(CAP_WEB_SVELTE_SVELTE_CONFIG),
        "apps/web/vite.config.ts" => Some(CAP_WEB_SVELTE_VITE_CONFIG),
        "apps/web/src/app.html" => Some(CAP_WEB_SVELTE_APP_HTML),
        "apps/web/src/app.css" => Some(CAP_WEB_SVELTE_APP_CSS),
        "apps/web/src/routes/+layout.svelte" => Some(CAP_WEB_SVELTE_LAYOUT),
        "apps/web/src/routes/+page.svelte" => Some(CAP_WEB_SVELTE_PAGE),
        // firmware-rp2040 capability
        "firmware/Cargo.toml" => Some(CAP_FIRMWARE_RP2040_WORKSPACE),
        "firmware/rust-toolchain.toml" => Some(CAP_FIRMWARE_RP2040_TOOLCHAIN),
        "firmware/shared/Cargo.toml" => Some(CAP_FIRMWARE_RP2040_SHARED_CARGO),
        "firmware/shared/src/lib.rs" => Some(CAP_FIRMWARE_RP2040_SHARED_LIB),
        "firmware/rp2040/.cargo/config.toml" => Some(CAP_FIRMWARE_RP2040_CARGO_CONFIG),
        "firmware/rp2040/Cargo.toml" => Some(CAP_FIRMWARE_RP2040_CARGO),
        "firmware/rp2040/build.rs" => Some(CAP_FIRMWARE_RP2040_BUILD),
        "firmware/rp2040/memory.x" => Some(CAP_FIRMWARE_RP2040_MEMORY),
        "firmware/rp2040/src/main.rs" => Some(CAP_FIRMWARE_RP2040_MAIN),
        "firmware/rp2040/README.md" => Some(CAP_FIRMWARE_README),
        // firmware-stm32 capability
        // Note: firmware/Cargo.toml and firmware/rust-toolchain.toml are shared
        // path keys; stm32 variants are referenced here under stm32-prefixed keys
        // so they are accessible for upgrade checks when both capabilities coexist.
        "firmware-stm32/Cargo.toml" => Some(CAP_FIRMWARE_STM32_WORKSPACE),
        "firmware-stm32/rust-toolchain.toml" => Some(CAP_FIRMWARE_STM32_TOOLCHAIN),
        "firmware-stm32/shared/Cargo.toml" => Some(CAP_FIRMWARE_STM32_SHARED_CARGO),
        "firmware-stm32/shared/src/lib.rs" => Some(CAP_FIRMWARE_STM32_SHARED_LIB),
        "firmware/stm32/.cargo/config.toml" => Some(CAP_FIRMWARE_STM32_CARGO_CONFIG),
        "firmware/stm32/Cargo.toml" => Some(CAP_FIRMWARE_STM32_CARGO),
        "firmware/stm32/build.rs" => Some(CAP_FIRMWARE_STM32_BUILD),
        "firmware/stm32/memory.x" => Some(CAP_FIRMWARE_STM32_MEMORY),
        "firmware/stm32/src/main.rs" => Some(CAP_FIRMWARE_STM32_MAIN),
        // tauri capability
        "apps/desktop/src-tauri/tauri.conf.json" => Some(CAP_TAURI_CONF),
        "apps/desktop/src-tauri/Cargo.toml" => Some(CAP_TAURI_CARGO),
        "apps/desktop/src-tauri/build.rs" => Some(CAP_TAURI_BUILD),
        "apps/desktop/src-tauri/src/lib.rs" => Some(CAP_TAURI_LIB),
        "apps/desktop/src-tauri/src/main.rs" => Some(CAP_TAURI_MAIN),
        // worker-cloudflare capability
        "apps/worker/wrangler.toml" => Some(CAP_WORKER_WRANGLER),
        // eda capability
        "board/main.ato" => Some(CAP_EDA_MAIN_ATO),
        "board/board.interface.json" => Some(CAP_EDA_BOARD_INTERFACE),
        "pipelines/eda.toml" => Some(CAP_EDA_PIPELINE),
        _ => None,
    }
}

/// Expand `{{name}}` and `{{version}}` placeholders in a raw template.
pub fn expand(raw_template: &str, product_name: &str, platform_version: &str) -> String {
    raw_template
        .replace("{{name}}", product_name)
        .replace("{{version}}", platform_version)
}
