#!/usr/bin/env bash
#
# The design system's portable exports are accepted by the tools they are for.
#
# `design-system.md` is derived into `registry.json` (a shadcn registry
# holding the theme) and `design/tokens.json` (W3C Design Tokens). A schema
# written from memory proves nothing, so this runs the real consumers:
#
#   * `shadcn build` accepts the registry and writes the installable item;
#   * `shadcn add` of the theme changes a product's stylesheet, and a second
#     product with none of it installs the whole system with `@demo/base`;
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
npx -y "shadcn@$SHADCN" build registry.json --output "$work/r" >/dev/null
test -s "$work/r/theme.json" || { echo "shadcn build wrote no item" >&2; exit 1; }

echo "  shadcn add (the item reaches a stylesheet)"
before="$(md5sum apps/web/src/app/globals.css | cut -d' ' -f1)"
(cd apps/web && npx -y "shadcn@$SHADCN" add "$work/r/theme.json" --yes --overwrite >/dev/null)
after="$(md5sum apps/web/src/app/globals.css | cut -d' ' -f1)"
[ "$before" != "$after" ] || { echo "installing the theme changed nothing" >&2; exit 1; }

echo "  a second product installs the whole system through the namespace"
# Serve the built registry and point a product that has none of it at the
# namespace, the way another product would. `{name}` is the item shadcn asks for.
port=8765
python3 -m http.server "$port" --directory "$work/r" >/dev/null 2>&1 &
server=$!
trap 'kill $server 2>/dev/null; rm -rf "$work"' EXIT
sleep 1
cp -r "$work/demo/apps/web" "$work/consumer"
cd "$work/consumer"
rm -rf src/components/ui src/app/tokens.css src/app/marks.css
sed -i '/tokens.css\|marks.css/d' src/app/globals.css
node -e '
const fs=require("fs");const c=JSON.parse(fs.readFileSync("components.json","utf8"));
c.registries={"@demo":"http://localhost:'"$port"'/{name}.json"};
fs.writeFileSync("components.json",JSON.stringify(c,null,2));'
npx -y "shadcn@$SHADCN" add @demo/base --yes --overwrite >/dev/null
for f in src/components/ui/{button,card,dropdown-menu,doodle-arrows}.tsx src/app/tokens.css src/app/marks.css; do
  [ -s "$f" ] || { echo "installing @demo/base did not write $f" >&2; exit 1; }
done
cmp src/components/ui/button.tsx "$work/demo/apps/web/src/components/ui/button.tsx" \
  || { echo "the installed button differs from the product's" >&2; exit 1; }
grep -q 'tokens.css' src/app/globals.css \
  || { echo "the system item did not import its tokens" >&2; exit 1; }
cd "$work"

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
