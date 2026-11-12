#!/usr/bin/env bash
# Which parts of the repository a change touches, as `group=true|false` lines.
#
#   scripts/ci-changes.sh <base-sha> <head-sha>     # one line per group
#   scripts/ci-changes.sh --all                     # everything true
#
# CI runs each job only when its group is true, so a change to a web template
# does not build firmware, and a change to a spine crate does not scaffold a
# Next.js app. The groups are declared here, once, as path patterns — not as a
# `paths:` filter repeated in every job, where the copy for one job would drift
# from the copy for the job that depends on the same code.
#
# Two rules keep this from hiding a failure:
#
#   · A change to CI itself (this script, the workflow, the toolchain pins)
#     turns every group on. A filter that can exclude its own definition can
#     be broken without anything running to notice.
#   · An unknown or unreadable base turns every group on. Running too much is a
#     cost; skipping a job that should have run is a defect that looks green.
set -euo pipefail

groups=(spine crates cli firmware hardware js identity adapters web docs)

all_true() {
  for g in "${groups[@]}"; do echo "$g=true"; done
}

if [ "${1:-}" = "--all" ] || [ $# -lt 2 ]; then
  all_true
  exit 0
fi

base="$1"
head="$2"
if ! files=$(git diff --name-only "$base" "$head" 2>/dev/null); then
  all_true
  exit 0
fi

touched() {
  # True when any changed file matches any of the given extended regexes.
  local pattern
  for pattern in "$@"; do
    if grep -Eq "$pattern" <<<"$files"; then return 0; fi
  done
  return 1
}

if touched '^\.github/workflows/ci\.yml$' '^scripts/ci-changes\.sh$' '^rust-toolchain\.toml$' '^\.nvmrc$'; then
  all_true
  exit 0
fi

cap='^crates/fiducial-cli/capabilities'
declare -A on

# The no_std crates everything else is built on.
touched '^crates/fiducial-(core|geometry|quantity|model|protocol|identity|ota|eda|mesh|sim|wasm|adapters|tauri)/' '^crates/fiducial/' '^Cargo\.(toml|lock)$' \
  && on[spine]=true
# Any Rust at all: the host build, the test suite, the release check.
touched '^crates/' '^Cargo\.(toml|lock)$' '^examples/' '^scripts/' '^thesis\.toml$' && on[crates]=true
# The CLI's own code, which generates what several other jobs compile.
touched '^crates/fiducial-cli/(src|tests|templates|build\.rs)' && on[cli]=true
touched "$cap/firmware-" '^crates/fiducial-(core|protocol|ota)/' && on[firmware]=true
touched "$cap/(hardware|eda)/" '^crates/fiducial-(mesh|eda|geometry)/' '^packages/(board-schema|viewer3d-react)/' '^crates/fiducial-cli/src/(hardware|eda|mesh)' \
  && on[hardware]=true
touched '^packages/' '^pnpm-lock\.yaml$' '^pnpm-workspace\.yaml$' '^package\.json$' '^turbo\.json$' && on[js]=true
touched '^packages/identity/' '^crates/fiducial-identity/' "$cap/identity/" '^crates/fiducial-cli/src/' && on[identity]=true
touched '^packages/adapters/' '^crates/fiducial-adapters/' "$cap/(adapters|migrations|deploy)/" '^crates/fiducial-cli/src/' && on[adapters]=true
touched "$cap/(web-next|design|i18n|content|cms|seo|brand|legal)/" '^crates/fiducial-cli/src/' '^scripts/verify-web-next\.sh$' && on[web]=true
touched '\.md$' '^docs/' '^crates/fiducial-cli/' && on[docs]=true

for g in "${groups[@]}"; do
  echo "$g=${on[$g]:-false}"
done
