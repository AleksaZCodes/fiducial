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
