#!/usr/bin/env bash
#
# A freshly scaffolded TanStack Start product installs, typechecks, lints, builds,
# deploys as a Worker (dry run), and serves every required security header from
# the built app, running in workerd.
#
# The Rust tests read the templates as text; only the real package manager and
# the real compiler prove they build (see verify-web-next.sh).
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-web-start.sh
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
pnpm=(npx --yes pnpm@11)

[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
server=""
cleanup() {
  [ -n "$server" ] && kill "$server" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

echo "── web-start ─────────────────────────────────────────────────────────"
cd "$work"
"$fid" new site >/dev/null
cd site
"$fid" add design >/dev/null
"$fid" add app start >/dev/null
"$fid" add deploy >/dev/null

cat >> fiducial.toml <<'TOML'

[apps.web]
framework = "tanstack-start"
TOML
sed -i 's/^\[deploy\]/[deploy]\nshape = "tanstackstart"/' fiducial.toml
grep -q '^shape = "tanstackstart"' fiducial.toml || printf '\n[deploy]\nshape = "tanstackstart"\n' >> fiducial.toml
sed -i 's/REPLACE_WITH_DATABASE_ID/00000000-0000-0000-0000-000000000000/; s/REPLACE_WITH_BUCKET_NAME/site-assets/; s/REPLACE_WITH_QUEUE_NAME/site-jobs/' fiducial.toml

"$fid" derive >/dev/null
if "$fid" doctor 2>&1 | grep -E "migration pending|upstream template updated|✗"; then
  echo "✗ a fresh TanStack Start product reports problems" >&2
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
"${pnpm[@]}" exec vite preview --port 4173 --strictPort >"$work/preview.log" 2>&1 &
server=$!
for _ in $(seq 1 60); do
  curl -fsS -o /dev/null "$base/" 2>/dev/null && break
  sleep 1
done
headers="$(curl -fsSI "$base/")"
for h in Strict-Transport-Security X-Content-Type-Options Referrer-Policy X-Frame-Options \
         Permissions-Policy Cross-Origin-Opener-Policy Content-Security-Policy; do
  echo "$headers" | grep -qi "^$h:" || { echo "✗ the response lacks $h" >&2; echo "$headers" >&2; exit 1; }
done
curl -fsS "$base/" | grep -q "Hello from the Worker" \
  || { echo "✗ the loader's data is not in the page" >&2; exit 1; }

cd ../..
if ! "$fid" doctor 2>&1 | grep -q "no platform-owned file has been modified"; then
  "$fid" doctor >&2 || true
  echo "✗ install or build modified a platform-owned file" >&2
  exit 1
fi
echo "✓ start: installs, typechecks, lints, builds, deploys (dry run), serves its headers"
