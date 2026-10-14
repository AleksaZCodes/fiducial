#!/usr/bin/env bash
#
# A freshly scaffolded Next.js product installs, typechecks and builds.
#
# Why this exists: on 2026-10-13 the first product scaffolded with
# `fid add design` + `fid add app next` could not run `next build`. The web
# template's own files were never compiled anywhere — the design capability's
# button and dropdown imported Radix with no Radix installed, `@/lib/utils`
# did not exist, the stylesheet imported Tailwind with Tailwind not installed,
# the i18n pipeline wrote its module where nothing imported it, and pnpm 11
# refused to run any script at all over an undecided install script. Every
# Rust test was green, because every one of them read the templates as text.
#
# `crates/fiducial-cli/tests/fresh_next_product.rs` keeps the checks that need
# no network. This is the rest: the real package manager, the real compiler,
# the real bundler, on the real output — once per icon library in $ICONS,
# because the platform's components are rewritten per library and a rewrite
# that only one library exercises is a rewrite the others have not proved.
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-web-next.sh                     # lucide and hugeicons
#   ICONS="lucide tabler hugeicons phosphor remixicon" scripts/verify-web-next.sh
#
# pnpm is pinned to 11 on purpose: it is the strictest about install scripts,
# and the version a person who installs pnpm today gets.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
pnpm=(npx --yes pnpm@11)
icons="${ICONS:-lucide hugeicons}"

[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

for lib in $icons; do
  echo "── web-next, iconLibrary = $lib ──────────────────────────────────────"
  dir="$work/$lib"
  mkdir -p "$dir"
  (
    cd "$dir"
    "$fid" new site >/dev/null
    cd site
    "$fid" add design >/dev/null
    "$fid" add app next --icons "$lib" >/dev/null
    "$fid" add i18n >/dev/null
    "$fid" add brand >/dev/null
    "$fid" derive >/dev/null

    # Doctor must have nothing to say about a tree it wrote a second ago.
    if "$fid" doctor 2>&1 | grep -E "migration pending|upstream template updated"; then
      echo "✗ a fresh product reports upgrades pending" >&2
      exit 1
    fi

    cd apps/web
    "${pnpm[@]}" install --fetch-timeout 300000
    "${pnpm[@]}" run typecheck
    # The app's own lint script, over the templates as installed (#99): it
    # failed on every fresh product, because the platform shipped no Biome
    # config and nothing here ran it.
    "${pnpm[@]}" run lint
    "${pnpm[@]}" run build

    # Installing and building must not have rewritten a file the platform
    # owns. pnpm 11 does exactly that to `pnpm-workspace.yaml` when a declared
    # range admits no release older than its `minimumReleaseAge`.
    cd ../..
    if ! "$fid" doctor 2>&1 | grep -q "no platform-owned file has been modified"; then
      "$fid" doctor >&2 || true
      echo "✗ install or build modified a platform-owned file" >&2
      exit 1
    fi
  )
  echo "✓ $lib: installs, typechecks, builds"
done
