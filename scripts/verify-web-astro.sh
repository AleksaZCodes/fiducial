#!/usr/bin/env bash
#
# A freshly scaffolded Astro product installs, typechecks, lints and builds,
# and wrangler accepts the deploy config derived for it.
#
# The Rust tests read the templates as text; only the real package manager and
# the real compiler prove they build (see verify-web-next.sh for why that
# matters). Astro's page is plain files, so the checks after the build are about
# what was written: every required response header is in the output, and the
# page ships no JavaScript it was not asked for.
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-web-astro.sh
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
pnpm=(npx --yes pnpm@11)

[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "── web-astro ──────────────────────────────────────────────────────────"
cd "$work"
"$fid" new site >/dev/null
cd site
"$fid" add design >/dev/null
"$fid" add app astro >/dev/null
"$fid" add deploy >/dev/null

# Declare the app and its deploy shape. `fid doctor` holds the two to agreeing.
cat >> fiducial.toml <<'TOML'

[apps.web]
framework = "astro"
TOML
sed -i 's/^\[deploy\]/[deploy]\nshape = "astro"/' fiducial.toml
grep -q '^shape = "astro"' fiducial.toml || printf '\n[deploy]\nshape = "astro"\n' >> fiducial.toml

"$fid" skills sync --install >/dev/null   # a real product installs the vendored skills its capabilities name
"$fid" derive >/dev/null
if "$fid" doctor 2>&1 | grep -E "migration pending|upstream template updated|✗.*apps"; then
  echo "✗ a fresh Astro product reports problems" >&2
  "$fid" doctor >&2 || true
  exit 1
fi

"${pnpm[@]}" install --fetch-timeout 300000
cd apps/web
"${pnpm[@]}" run typecheck
"${pnpm[@]}" run lint
"${pnpm[@]}" run build

[ -f dist/index.html ] || { echo "✗ build wrote no dist/index.html" >&2; exit 1; }
[ -f dist/_headers ] || { echo "✗ _headers did not reach the build output" >&2; exit 1; }
if grep -rq "<script" dist/index.html; then
  echo "✗ the index page ships a script it was not asked for" >&2
  exit 1
fi

# wrangler must accept the derived config: static assets, no Worker entry.
"${pnpm[@]}" exec wrangler deploy --dry-run --config wrangler.jsonc

cd ../..
if ! "$fid" doctor 2>&1 | grep -q "no platform-owned file has been modified"; then
  "$fid" doctor >&2 || true
  echo "✗ install or build modified a platform-owned file" >&2
  exit 1
fi
echo "✓ astro: installs, typechecks, lints, builds, deploys (dry run)"

# The capability components for Astro (`for/web-astro/`): add i18n, content and
# press, put every component on a page, and make the real compiler accept them.
# This is the part the Rust tests cannot prove: that the `.astro` files parse,
# type-check against the generated modules, and build.
echo "── web-astro components ───────────────────────────────────────────────"
"$fid" add capability i18n >/dev/null
"$fid" add capability content >/dev/null
"$fid" add capability press >/dev/null
"$fid" derive >/dev/null
cat > apps/web/src/pages/components.astro <<'ASTRO'
---
import EntryCards from "../components/EntryCards.astro";
import LocalePicker from "../components/LocalePicker.astro";
import MobileMenu from "../components/MobileMenu.astro";
import PressKit from "../components/PressKit.astro";
import { content } from "../generated/content";
import { defaultLocale } from "../generated/messages";
import Base from "../layouts/Base.astro";
import { formatContentDate } from "../lib/intl";

const locale = defaultLocale;
const posts = content.collections.posts.byLocale[locale] ?? [];
---

<Base title="Components">
  <MobileMenu links={[{ href: "/", label: "Home" }]} openLabel="Menu" closeLabel="Close" />
  <LocalePicker locale={locale} url={Astro.url} />
  <EntryCards entries={posts} locale={locale} href={(e) => `/blog/${e.slug}`} readMoreLabel="Read more" />
  <PressKit
    locale={locale}
    labels={{ title: "Press", assets: "Assets", guidelines: "Use as is", facts: "Facts", boilerplate: "Boilerplate", short: "Short", medium: "Medium", long: "Long", stories: "Stories", contact: "Contact", download: "Download" }}
    logos={[{ label: "Mark", href: "/favicon.svg" }]}
    storyHref={(s) => `/press/${s}`}
    formatDate={(iso) => formatContentDate(iso, locale)}
  />
</Base>
ASTRO
"${pnpm[@]}" install --fetch-timeout 300000
cd apps/web
"${pnpm[@]}" run typecheck
"${pnpm[@]}" run build
for marker in 'data-fid-locale' 'data-fid-menu' 'fid-cover' 'class="kit"'; do
  grep -q "$marker" dist/components/index.html || { echo "✗ components page lacks $marker" >&2; exit 1; }
done
echo "✓ astro: the i18n, content and press components typecheck and build"
