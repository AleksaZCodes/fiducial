#!/usr/bin/env bash
#
# A freshly scaffolded Hono API, with Drizzle over it, installs, typechecks,
# lints, passes its tests, deploys as a Worker (dry run), and turns a schema
# change into a migration the platform's runner will apply.
#
# The Rust tests read the templates as text; only the real package manager and
# the real compiler prove they work (see verify-web-next.sh).
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-api-hono.sh
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
pnpm=(npx --yes pnpm@11)

[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "── api-hono + orm-drizzle ─────────────────────────────────────────────"
cd "$work"
"$fid" new svc >/dev/null
cd svc
"$fid" add app api >/dev/null
"$fid" add deploy >/dev/null
"$fid" add capability migrations >/dev/null
"$fid" add capability orm-drizzle >/dev/null

# The Worker config is derived into the API app, and the product says what the
# API is and what it uses: doctor holds those to being buildable and consistent.
cat > pipelines/deploy.toml <<'TOML'
name     = "deploy"
executor = "fid-deploy"
outputs  = ["apps/api/wrangler.toml"]
TOML
cat >> fiducial.toml <<'TOML'

[adapters]
database = "d1"

[apps.api]
framework = "hono"
orm       = "drizzle"
TOML
sed -i 's/REPLACE_WITH_DATABASE_ID/00000000-0000-0000-0000-000000000000/; s/REPLACE_WITH_BUCKET_NAME/svc-assets/; s/REPLACE_WITH_QUEUE_NAME/svc-jobs/' fiducial.toml

"$fid" derive >/dev/null
if "$fid" doctor 2>&1 | grep -E "migration pending|upstream template updated|✗"; then
  echo "✗ a fresh API product reports problems" >&2
  "$fid" doctor >&2 || true
  exit 1
fi

"${pnpm[@]}" install --fetch-timeout 300000
cd apps/api
"${pnpm[@]}" run typecheck
"${pnpm[@]}" run lint
"${pnpm[@]}" run test
"${pnpm[@]}" exec wrangler deploy --dry-run --config wrangler.toml

# A schema change becomes a migration, and the platform's manifest carries it.
cat > src/db/schema.ts <<'TS'
import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";

export const notes = sqliteTable("notes", {
  id: integer("id").primaryKey({ autoIncrement: true }),
  body: text("body").notNull(),
});
TS
"${pnpm[@]}" run db:generate
cd ../..
ls migrations/*.sql >/dev/null || { echo "✗ db:generate wrote no migration" >&2; exit 1; }
grep -q "CREATE TABLE \`notes\`" migrations/*.sql || { echo "✗ the migration lacks the table" >&2; exit 1; }
"$fid" derive >/dev/null
grep -q "notes" src/migrations.generated.ts || { echo "✗ the manifest lacks the migration" >&2; exit 1; }
"$fid" derive --check >/dev/null

# Generating again with no change must write nothing: the schema and the SQL agree.
before="$(ls migrations/*.sql | wc -l)"
(cd apps/api && "${pnpm[@]}" run db:generate >/dev/null)
after="$(ls migrations/*.sql | wc -l)"
[ "$before" = "$after" ] || { echo "✗ an unchanged schema generated another migration" >&2; exit 1; }

# The typed queries still compile against the real schema.
(cd apps/api && "${pnpm[@]}" run typecheck)

if ! "$fid" doctor 2>&1 | grep -q "no platform-owned file has been modified"; then
  "$fid" doctor >&2 || true
  echo "✗ install or build modified a platform-owned file" >&2
  exit 1
fi
echo "✓ api: installs, typechecks, lints, tests, deploys (dry run), generates and derives a migration"
