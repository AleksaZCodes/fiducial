#!/usr/bin/env bash
#
# The design system's portable exports are accepted by the tools they are for.
#
# `design-system.md` is derived into `design/registry.json` (a shadcn registry
# holding the theme) and `design/tokens.json` (W3C Design Tokens). A schema
# written from memory proves nothing, so this runs the real consumers:
#
#   * `shadcn build` accepts the registry and writes the installable item;
#   * `shadcn add` of that item changes a product's stylesheet;
#   * `[design.registries]` lands in components.json, where shadcn reads it;
#   * Style Dictionary reads tokens.json and emits a colour.
#
# It runs weekly as well as on changes, because shadcn can tighten its schema
# without a commit here (it started requiring a root `homepage` that way).
#
# Needs Node 22+ and the network.
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-design-registry.sh
set -euo pipefail

SHADCN="${SHADCN_VERSION:-4.23.0}"
SD="${STYLE_DICTIONARY_VERSION:-5.8.0}"

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cd "$work"

echo "── design registry ────────────────────────────────────────────────────"
"$fid" new demo >/dev/null
cd demo
"$fid" add design >/dev/null
"$fid" add app next >/dev/null
cat >> fiducial.toml <<'TOML'

[design.registries]
"@acme" = "https://acme.example/r/{name}.json"
TOML
"$fid" derive >/dev/null
"$fid" derive --check >/dev/null

grep -q '"@acme"' apps/web/components.json \
  || { echo "[design.registries] did not reach components.json" >&2; exit 1; }

echo "  shadcn build"
npx -y "shadcn@$SHADCN" build design/registry.json --output "$work/r" >/dev/null
test -s "$work/r/demo-theme.json" || { echo "shadcn build wrote no theme item" >&2; exit 1; }

echo "  shadcn add (the item reaches a stylesheet)"
before="$(md5sum apps/web/src/app/globals.css | cut -d' ' -f1)"
(cd apps/web && npx -y "shadcn@$SHADCN" add "$work/r/demo-theme.json" --yes --overwrite >/dev/null)
after="$(md5sum apps/web/src/app/globals.css | cut -d' ' -f1)"
[ "$before" != "$after" ] || { echo "installing the theme changed nothing" >&2; exit 1; }

echo "  style-dictionary (tokens.json)"
mkdir "$work/sd" && cd "$work/sd"
npm init -y >/dev/null
npm i --silent "style-dictionary@$SD" >/dev/null
cat > build.mjs <<JS
import StyleDictionary from 'style-dictionary';
const sd = new StyleDictionary({
  source: ['$work/demo/design/tokens.json'],
  usesDtcg: true,
  platforms: { css: { transformGroup: 'css', buildPath: 'out/', files: [{ destination: 'v.css', format: 'css/variables' }] } },
});
await sd.buildAllPlatforms();
JS
node build.mjs >/dev/null
grep -q -- '--color-light-background' out/v.css \
  || { echo "style-dictionary did not read the colours" >&2; exit 1; }

echo "✓ the registry builds and installs; tokens.json is read by Style Dictionary"
