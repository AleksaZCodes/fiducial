#!/usr/bin/env bash
#
# A freshly scaffolded SvelteKit product installs, typechecks, lints, builds
# for Cloudflare, and has its derived Worker config accepted by wrangler.
#
# `crates/fiducial-cli/tests/svelte_product.rs` keeps the checks that need no
# network. Until this existed the SvelteKit templates had never been compiled:
# the stylesheet imported Tailwind with no Tailwind installed, `svelte-check`
# could not run on the pinned TypeScript, and `adapter-auto` built nothing a
# Cloudflare deploy could ship.
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-web-svelte.sh
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
pnpm=(npx --yes pnpm@11)
[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work"
"$fid" new site --locales en,sr --default-locale en >/dev/null
cd site
for step in "add design" "add app svelte" "add brand" "add legal" "add capability content" \
  "add capability seo" "add capability cms" "add capability press" "add deploy"; do
  # shellcheck disable=SC2086
  "$fid" $step >/dev/null
done
"$fid" derive >/dev/null
"$fid" derive --check
"${pnpm[@]}" install --fetch-timeout 300000
cd apps/web
"${pnpm[@]}" run typecheck
"${pnpm[@]}" run lint
"${pnpm[@]}" run build
"${pnpm[@]}" exec wrangler deploy --dry-run
cd ../..
if ! "$fid" doctor 2>&1 | grep -q "no platform-owned file has been modified"; then
  "$fid" doctor >&2 || true
  echo "✗ install or build modified a platform-owned file" >&2
  exit 1
fi
echo "✓ sveltekit: installs, typechecks, lints, builds for Workers, deploys (dry run)"
