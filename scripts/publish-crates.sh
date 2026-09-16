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
# ## The rate limit, and why this loops
#
# crates.io allows a burst of **5 brand-new crates**, then **1 per 10 minutes**
# (new *versions* of crates that already exist get a burst of 30, then 1 per
# minute — a different, much looser bucket). The first real run of this script
# published exactly five and then took a 429 naming the time to retry:
#
#   error: failed to publish fiducial-protocol v0.1.0
#   Caused by: the remote server responded with an error (status 429 Too Many
#   Requests): You have published too many new crates in a short period of
#   time. Please try again after Wed, 16 Sep 2026 14:38:58 GMT
#
# So this waits until the time the server named and goes round again. Each pass
# re-asks crates.io what is missing, which makes the loop converge whatever
# happened in the previous pass — a partial publish is just a shorter next one.
#
# The wait is bounded by `PUBLISH_MAX_WAIT_SECS` (default two hours) so a
# pathological case cannot burn a six-hour runner. Reaching the cap fails,
# because a publish that did not finish is not a publish that succeeded.
#
# Note this path costs an hour and a half *once*. After the first release every
# crate exists, so every later run publishes new versions into the burst of 30
# and never sleeps at all. That is why the wait lives here rather than being
# engineered away: it is a real condition, rarely hit, and the honest handling
# is to wait exactly as long as the server asked.
#
# Usage:
#
#   scripts/publish-crates.sh              # publish what is missing
#   scripts/publish-crates.sh --dry-run    # say what would be published
#
# `--dry-run` runs cargo's own `--dry-run`, so it still packages and verifies
# each crate — it is a real check, not a printout. It never sleeps.
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

max_wait=${PUBLISH_MAX_WAIT_SECS:-7200}
waited=0

# What crates.io does not have yet, as `--package NAME` arguments.
#
# Re-derived on every pass rather than computed once: after a partial publish
# the answer has changed, and asking again is both cheaper and more truthful
# than tracking it ourselves.
missing_args() {
  local name version code
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
        # Anything else is an unknown state, and publishing into an unknown
        # state is how you find out that 403 meant "no User-Agent" the hard way.
        echo "crates.io returned $code checking $name $version" >&2
        exit 1
        ;;
    esac
  done < <(cargo metadata --no-deps --format-version 1 \
    | jq -r '.packages[] | select(.publish != []) | [.name, .version] | @tsv' | sort)
}

log=$(mktemp)
trap 'rm -f "$log"' EXIT

while :; do
  missing_args

  if [ ${#args[@]} -eq 0 ]; then
    echo "✦ all $skipped crate(s) already on crates.io — nothing left to publish"
    exit 0
  fi

  echo
  # `args` holds a `--package NAME` pair per crate, so the crate count is half
  # its length. Printing the raw length claimed thirty crates for fifteen.
  echo "publishing $(( ${#args[@]} / 2 )) crate(s); cargo orders them by dependency"

  # Not `--workspace`: that would include the already-published ones and fail.
  if cargo publish $dry_run "${args[@]}" 2>&1 | tee "$log"; then
    echo "✦ publish pass completed"
    exit 0
  fi

  if [ -n "$dry_run" ]; then
    echo "✗ dry run failed — see above" >&2
    exit 1
  fi

  # crates.io names the time to retry. Honour it rather than guessing at the
  # throttle: the bucket refills on its schedule, not ours.
  retry_at=$(sed -n 's/.*[Pp]lease try again after \([^)]*\) and see.*/\1/p' "$log" | head -1)
  if [ -z "$retry_at" ]; then
    echo "✗ publish failed, and not for a rate limit — see the error above" >&2
    exit 1
  fi

  now=$(date -u +%s)
  until=$(date -u -d "$retry_at" +%s 2>/dev/null || echo 0)
  # +15s of slack: the server's clock and ours are not the same clock, and
  # retrying one second early spends a whole window to learn that.
  sleep_for=$(( until - now + 15 ))
  [ "$sleep_for" -lt 15 ] && sleep_for=15

  waited=$(( waited + sleep_for ))
  if [ "$waited" -gt "$max_wait" ]; then
    echo "✗ rate limited, and waiting further would exceed PUBLISH_MAX_WAIT_SECS=$max_wait" >&2
    echo "  Crates still missing are listed above. Re-run to continue." >&2
    exit 1
  fi

  echo
  echo "⏳ rate limited by crates.io; it asked to retry after $retry_at"
  echo "   sleeping ${sleep_for}s (${waited}s of ${max_wait}s budget used), then re-checking"
  sleep "$sleep_for"
done
