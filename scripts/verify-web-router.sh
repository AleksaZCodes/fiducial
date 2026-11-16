#!/usr/bin/env bash
#
# A freshly scaffolded React Router product installs, typechecks, lints, builds,
# deploys as a Worker (dry run), and serves every required security header from
# the built app, running in workerd.
#
# The Rust tests read the templates as text; only the real package manager and
# the real compiler prove they build (see verify-web-next.sh).
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-web-router.sh
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
pnpm=(npx --yes pnpm@11)

[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
server=""
cleanup() {
  # The preview is `npx pnpm exec vite preview`: killing the wrapper alone leaves
  # vite and workerd running, still holding the port. It runs in its own process
  # group (setsid), and the whole group goes.
  [ -n "$server" ] && kill -- "-$server" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

echo "── web-router ─────────────────────────────────────────────────────────"
cd "$work"
"$fid" new site >/dev/null
cd site
"$fid" add design >/dev/null
"$fid" add app router >/dev/null
"$fid" add deploy >/dev/null

cat >> fiducial.toml <<'TOML'

[apps.web]
framework = "react-router"
TOML
sed -i 's/^\[deploy\]/[deploy]\nshape = "reactrouter"/' fiducial.toml
grep -q '^shape = "reactrouter"' fiducial.toml || printf '\n[deploy]\nshape = "reactrouter"\n' >> fiducial.toml
sed -i 's/REPLACE_WITH_DATABASE_ID/00000000-0000-0000-0000-000000000000/; s/REPLACE_WITH_BUCKET_NAME/site-assets/; s/REPLACE_WITH_QUEUE_NAME/site-jobs/' fiducial.toml

"$fid" skills sync --install >/dev/null   # a real product installs the vendored skills its capabilities name
"$fid" derive >/dev/null
if "$fid" doctor 2>&1 | grep -E "migration pending|upstream template updated|✗"; then
  echo "✗ a fresh React Router product reports problems" >&2
  "$fid" doctor >&2 || true
  exit 1
fi

"${pnpm[@]}" install --fetch-timeout 300000
cd apps/web
"${pnpm[@]}" run typecheck
"${pnpm[@]}" run lint
"${pnpm[@]}" run build
"${pnpm[@]}" exec wrangler deploy --dry-run

# The built app, in workerd: every required header is on a real response.
# curl treats a bare host:port as plain http.
base="localhost:4173"
# One port per script, and a port nobody holds: a server left over from another
# run answers a request, and a test that cannot tell whose answer it is is not a test.
if curl -fsS -o /dev/null "$base/" 2>/dev/null; then
  echo "✗ something already answers on $base" >&2
  exit 1
fi
setsid "${pnpm[@]}" exec vite preview --port 4173 --strictPort >"$work/preview.log" 2>&1 &
server=$!
for _ in $(seq 1 60); do
  kill -0 "$server" 2>/dev/null || { echo "✗ the preview server exited" >&2; cat "$work/preview.log" >&2; exit 1; }
  curl -fsS -o /dev/null "$base/" 2>/dev/null && break
  sleep 1
done
headers="$(curl -fsSI "$base/")"
for h in Strict-Transport-Security X-Content-Type-Options Referrer-Policy X-Frame-Options \
         Permissions-Policy Cross-Origin-Opener-Policy Content-Security-Policy; do
  echo "$headers" | grep -qi "^$h:" || { echo "✗ the response lacks $h" >&2; echo "$headers" >&2; exit 1; }
done
# The full-stack loop, with JavaScript out of it: the loader's data is in the
# page, a form POST reaches the action, and bad input comes back as a 400.
curl -fsS "$base/" | grep -q "<h1>site</h1>" \
  || { echo "✗ the loader's data is not in the page" >&2; exit 1; }
# A POST to "/" would target the root route, which has no action: the index
# route's action is addressed as "/?index", and that is what the rendered <form>
# posts to. Read the address from the page, so the test follows what a browser does.
action="$(curl -fsS "$base/" | grep -o '<form[^>]*' | grep -o 'action="[^"]*"' | head -1 | sed 's/action="//; s/"$//; s/&amp;/\&/g')"
[ "$action" = "/?index" ] || { echo "✗ the form posts to '$action', not '/?index'" >&2; exit 1; }
curl -fsS -d "name=Ada" "$base$action" | grep -q "Hello, Ada" \
  || { echo "✗ the action did not answer a form POST" >&2; exit 1; }
code="$(curl -sS -o "$work/bad.html" -w '%{http_code}' -d "name=" "$base$action")"
[ "$code" = 400 ] || { echo "✗ an empty name returned $code, not 400" >&2; exit 1; }
grep -q "Tell us your name" "$work/bad.html" \
  || { echo "✗ the validation message is not in the response" >&2; exit 1; }
# A resource route is JSON with no component.
curl -fsS "$base/healthz" | grep -q '"ok":true' \
  || { echo "✗ the resource route did not answer" >&2; exit 1; }
# Server-only code stays on the server: the browser bundle must not carry it.
if grep -rq "rendered on the Worker" build/client; then
  echo "✗ a .server module reached the client bundle" >&2
  exit 1
fi

cd ../..
if ! "$fid" doctor 2>&1 | grep -q "no platform-owned file has been modified"; then
  "$fid" doctor >&2 || true
  echo "✗ install or build modified a platform-owned file" >&2
  exit 1
fi
echo "✓ router: installs, typechecks, lints, builds, deploys (dry run), serves its headers"
