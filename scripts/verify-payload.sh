#!/usr/bin/env bash
#
# A freshly scaffolded Payload product installs, typechecks, builds for
# Workers and passes `wrangler deploy --dry-run`.
#
# The Rust test suite (`tests/payload_capability.rs`) reads the templates as
# text: it cannot prove the product compiles.  This script does: it scaffolds
# the same tree a developer gets, installs real packages, runs the TypeScript
# compiler, builds the Worker bundle with OpenNext and has Wrangler accept the
# config — without a live D1, a Cloudflare token or a browser.
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   TMPDIR=$PWD/target/tmp scripts/verify-payload.sh
#
# TMPDIR must be on the same filesystem as the work directory.  A Payload
# `node_modules` placed in a tmpfs /tmp runs out of space mid-build.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
pnpm=(npx --yes pnpm@11)

[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "── payload: scaffold ───────────────────────────────────────────────────"
(
  cd "$work"
  "$fid" new site --shape site >/dev/null
  cd site
  "$fid" add capability payload >/dev/null
  "$fid" derive >/dev/null

  "$fid" skills sync --install >/dev/null
  if "$fid" doctor 2>&1 | grep -E "migration pending|upstream template updated"; then
    echo "✗ a fresh payload product reports upgrades pending" >&2
    exit 1
  fi

  cd apps/web
  "${pnpm[@]}" install --fetch-timeout 300000
  cd ../..
  "$fid" derive --pipeline payload-codegen
  cd apps/web
  "${pnpm[@]}" run typecheck
  "${pnpm[@]}" run lint
  "${pnpm[@]}" exec opennextjs-cloudflare build
  "${pnpm[@]}" exec wrangler deploy --dry-run

  cd ../..
  if ! "$fid" doctor 2>&1 | grep -q "no platform-owned file has been modified"; then
    "$fid" doctor >&2 || true
    echo "✗ install or build modified a platform-owned file" >&2
    exit 1
  fi
)
echo "✓ payload: installs, typechecks, builds for Workers, deploys (dry run)"

echo "── commerce: scaffold ──────────────────────────────────────────────────"
(
  cd "$work"
  "$fid" new cshop >/dev/null
  cd cshop
  for cap in design web-next i18n brand payload commerce; do
    "$fid" add capability "$cap" >/dev/null
  done

  cfg="$PWD/fiducial.toml"
  content="$(cat "$cfg")"
  content="${content//REPLACE_WITH_DATABASE_ID/db-id-2}"
  content="${content//REPLACE_WITH_BUCKET_NAME/cshop-assets}"
  content="${content//REPLACE_WITH_QUEUE_NAME/cshop-jobs}"
  content="${content/\[i18n\]/[i18n]
same = ["commerce.guestEmail","commerce.leadSourceFacebook","commerce.leadSourceInstagram","payload.adminTitle","payload.emailLabel"]}"
  printf '%s\n' "$content" > "$cfg"

  "$fid" derive >/dev/null

  for f in \
    apps/web/src/app/api/orders/route.ts \
    apps/web/src/app/api/quotes/route.ts \
    apps/web/src/components/commerce/VariantPicker.tsx \
    apps/web/src/components/commerce/OrderForm.tsx \
    apps/web/src/components/commerce/QuoteForm.tsx; do
    [ -f "$f" ] || { echo "✗ missing commerce template: $f" >&2; exit 1; }
  done

  cd apps/web
  "${pnpm[@]}" install --fetch-timeout 300000
  "${pnpm[@]}" run typecheck
)
echo "✓ commerce: templates present, installs, typechecks"
