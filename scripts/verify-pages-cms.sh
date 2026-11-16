#!/usr/bin/env bash
#
# The Pages CMS config derived for a product is accepted by Pages CMS's own
# schema.
#
# Pages CMS ignores a malformed `.pages.yml` without an error, so nothing else
# would notice an emitter that is wrong. This builds a product that uses every
# kind of field the model has — text, long text, dates, partial dates, numbers
# and number choices, booleans, lists, a choice, several choices, a relation,
# a list of records, a media key, a body, settings and interface text — derives,
# and runs the result through the application's schema twice: once with the
# picture as a text field (a product that stores media keys) and once with the
# picture picker (a product that stores URL paths and says where they live).
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-pages-cms.sh
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
# The commit of github.com/pages-cms/pages-cms whose `lib/config-schema.ts` this
# was written against. Bump it deliberately: a newer schema may reject what an
# older emitter wrote, and that is what this is here to find.
pin=6f4e860a35d934406580287e7042e5e111e207a1

[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "── pages-cms ──────────────────────────────────────────────────────────"
mkdir "$work/check"
cd "$work/check"
echo '{"type":"module","private":true}' > package.json
npm install --silent zod@3 yaml@2 >/dev/null
curl -fsSL "https://raw.githubusercontent.com/pages-cms/pages-cms/$pin/lib/config-schema.ts" -o schema.src.ts
# The schema asks the field registry which types exist; the registry pulls in the
# application's React. Its answer is the names registered in fields/registry.ts.
types='"boolean","code","date","file","image","number","reference","rich-text","select","string","text","uuid"'
sed "s#import { fieldTypes } from \"@/fields/registry\";#const fieldTypes = new Set([$types]);#" schema.src.ts > schema.ts
grep -q 'const fieldTypes = new Set' schema.ts || { echo "the schema no longer imports the registry the way this stub expects" >&2; exit 1; }

cd "$work"
"$fid" new p --locales en,sr --default-locale en >/dev/null
cd p
"$fid" add capability content >/dev/null
"$fid" add capability cms >/dev/null

cat >> content.toml <<'TOML'

[collections.people]
kind = "collection"
schema = { name = "string" }
body = "none"
route = ""

[collections.works]
kind = "collection"
schema = { title = "string", started = "partial-date", when = "date", tier = "number", live = "boolean", tags = "string[]", status = "string", kinds = "string[]", cover = "media", relatedPeople = "string[]", links = "object[]" }
body = "rich"
route = ""

[collections.works.objects]
links = { label = "string", url = "string" }
TOML
cat >> fiducial.toml <<'TOML'

[cms.selects]
"works.status" = ["active", "parked"]
"works.kinds" = ["a", "b"]
"works.tier" = [1, 2, 3]
TOML
for l in en sr; do
  mkdir -p "content/people/$l" "content/works/$l"
  printf -- '---\nname: Ada\n---\n' > "content/people/$l/ada.md"
  printf -- '---\ntitle: A\nstarted: "2026-11"\nwhen: 2026-01-02\ntier: 2\nlive: true\ntags: [x]\nstatus: active\nkinds: [a]\ncover: w/a/c.png\nrelatedPeople: [ada]\nlinks:\n  - label: L\n    url: u\n---\nBody\n' > "content/works/$l/a.md"
done
echo '{ "nav": { "home": "Home" }, "limit": 3, "on": true, "tags": ["a"] }' > settings.json

cp "$repo/scripts/pages-cms-check.mjs" "$work/check/pages-cms-check.mjs"
check() { (cd "$work/check" && node pages-cms-check.mjs schema.ts "$work/p/.pages.yml"); }

"$fid" derive >/dev/null
echo "· media keys (the picture is a text field)"
check
if grep -q 'type: "image"' .pages.yml; then
  echo "an image field, for a product that stores media keys" >&2
  exit 1
fi

echo "· URL paths (the picture is a picker)"
sed -i 's|^media_dir = "media"$|media_dir = "media"\npublic_base = "/images"\nmedia_input = "public/images"|' fiducial.toml
grep -q '^media_input' fiducial.toml || { echo "could not set media_input" >&2; exit 1; }
"$fid" derive >/dev/null
check
grep -q 'type: "image"' .pages.yml || { echo "no image field, for a product that stores URL paths" >&2; exit 1; }

echo "· a hand edit is caught by the gate"
echo '# edited' >> .pages.yml
if "$fid" derive --check >/dev/null 2>&1; then
  echo "a hand-edited .pages.yml passed --check" >&2
  exit 1
fi
echo "pages-cms: ok"
