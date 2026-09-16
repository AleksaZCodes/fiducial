#!/usr/bin/env bash
#
# Publish every workspace crate that is not already on crates.io.
#
# Why this exists, and why it is not a `for crate in …` loop:
#
# `release.yml` carried `for crate in fiducial; do` — one of fifteen — and that
# one was never published either. Hand-maintaining a list of fifteen names in
# dependency order is the second declaration this platform exists to delete,
# and it goes wrong the first time a crate is added. `cargo metadata` already
# holds the graph.
#
# Since Rust 1.90, `cargo publish` accepts several `--package` flags and
# publishes them **in dependency order**, waiting for each to appear in the
# index before the crates that depend on it. That is the hard half of the job,
# and it is cargo's, not this script's. What is left is deciding *which*
# packages to name — and that decision must come from the registry, because a
# `cargo publish` of an already-published version is an error, not a no-op.
#
# Usage:
#
#   scripts/publish-crates.sh              # publish what is missing
#   scripts/publish-crates.sh --dry-run    # say what would be published
#
# `--dry-run` runs cargo's own `--dry-run`, so it still packages and verifies
# each crate — it is a real check, not a printout.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"

dry_run=""
if [ "${1:-}" = "--dry-run" ]; then
  dry_run="--dry-run"
elif [ -n "${1:-}" ]; then
  echo "unknown argument '$1' — the only option is --dry-run" >&2
  exit 2
fi

# crates.io rejects requests without a User-Agent with 403, which would make a
# naive check report "not published" for every crate and then fail on the
# publish itself. The header is required, not decorative.
ua="fiducial-release (https://github.com/AleksaZCodes/fiducial)"

args=()
skipped=0

while IFS=$'\t' read -r name version; do
  code=$(curl -sS -H "User-Agent: $ua" -o /dev/null -w '%{http_code}' \
    "https://crates.io/api/v1/crates/$name/$version")
  case "$code" in
    200)
      echo "  = $name $version already on crates.io"
      skipped=$((skipped + 1))
      ;;
    404)
      echo "  + $name $version will be published"
      args+=(--package "$name")
      ;;
    *)
      # Anything else is an unknown state, and publishing into an unknown state
      # is how you find out that 403 meant "no User-Agent" the hard way.
      echo "crates.io returned $code checking $name $version" >&2
      exit 1
      ;;
  esac
done < <(cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.publish != []) | [.name, .version] | @tsv' | sort)

if [ ${#args[@]} -eq 0 ]; then
  echo "✦ all $skipped crate(s) already on crates.io — nothing to publish"
  exit 0
fi

echo
# `args` holds a `--package NAME` pair per crate, so the crate count is half
# its length. Printing the raw length claimed thirty crates for fifteen.
echo "publishing $(( ${#args[@]} / 2 )) crate(s); cargo orders them by dependency"
# Not `--workspace`: that would include the already-published ones and fail.
cargo publish $dry_run "${args[@]}"
