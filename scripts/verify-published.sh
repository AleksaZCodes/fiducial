#!/usr/bin/env bash
#
# Did the release actually reach the registries?
#
# Why this exists: on 2026-09-16 `changeset publish` printed
#
#   Successfully published: @fiducial/adapters@0.2.0, @fiducial/identity@0.2.0
#
# and exited 0. Only `identity` reached npm — `adapters` sat at 0.1.0 with an
# unchanged `time.modified` until the workflow was re-run. The same run printed
# `Creating git tags… Created git tags.` while `git ls-remote --tags origin`
# stayed empty afterwards: the tags were created in the runner's clone and
# never pushed.
#
# A publish step that exits 0 is not evidence anything was published, and a
# step that says it created a tag is not evidence a tag exists. This asks the
# registries and the remote instead, and fails when the answer disagrees with
# what the repository says should be there.
#
# Usage:
#
#   scripts/verify-published.sh            # verify npm, crates.io and tags
#   scripts/verify-published.sh npm        # one section only
#   scripts/verify-published.sh crates
#   scripts/verify-published.sh tags
#
# Exits non-zero naming every package or tag that is missing. Read-only: it
# publishes nothing and pushes nothing.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"

. "$repo/scripts/lib/json.sh"

# crates.io rejects requests without a User-Agent with 403, which would make a
# naive check report "not published" for every crate. The header is required,
# not decorative.
ua="fiducial-release (https://github.com/AleksaZCodes/fiducial)"

LEGACY_FILE="docs/release/legacy-untagged.txt"

section="${1:-all}"
missing=()

# Package versions come from the **committed** tree, never the working tree.
#
# `changesets/action` runs `changeset version` in place before opening its
# Release PR, so by the time this script runs in `release.yml` the working
# tree holds the *next* versions — ones deliberately not published yet. The
# first run after this script landed duly reported
# `@fiducial/adapters@0.3.0 — not on npm`, which was true and not a problem:
# 0.3.0 was sitting in an unmerged Release PR.
#
# HEAD is the right question in both cases. On an ordinary merge it holds the
# versions already released; on a Release PR merge it holds the versions this
# very run publishes.
committed() {
  git show "HEAD:$1" 2>/dev/null
}

# ── npm ───────────────────────────────────────────────────────────────────────
# Every workspace package that is not `private` is expected on the registry at
# the version its package.json declares. `pnpm list` is not used: it reports
# what is installed locally, which is exactly the thing that was already
# believed and already wrong.
verify_npm() {
  echo "── npm ──"
  local pkg name version private code
  while IFS= read -r pkg; do
    json=$(committed "$pkg") || continue
    [ -z "$json" ] && continue
    IFS=$'\t' read -r name version private < <(json_package_fields <<<"$json")
    [ "$private" = "true" ] && continue
    code=$(curl -sS -o /dev/null -w '%{http_code}' \
      "https://registry.npmjs.org/${name//\//%2f}/$version")
    case "$code" in
      200) printf '  ✓ %s@%s\n' "$name" "$version" ;;
      404) printf '  ✗ %s@%s — not on npm\n' "$name" "$version"
           missing+=("npm:$name@$version") ;;
      *)   printf '  ? %s@%s — registry returned %s\n' "$name" "$version" "$code"
           missing+=("npm:$name@$version (HTTP $code)") ;;
    esac
  done < <(find packages -maxdepth 2 -name package.json -not -path '*/node_modules/*' | sort)
}

# ── crates.io ─────────────────────────────────────────────────────────────────
verify_crates() {
  echo "── crates.io ──"
  local name version code
  while IFS=$'\t' read -r name version; do
    code=$(curl -sS -H "User-Agent: $ua" -o /dev/null -w '%{http_code}' \
      "https://crates.io/api/v1/crates/$name/$version")
    case "$code" in
      200) printf '  ✓ %s %s\n' "$name" "$version" ;;
      404) printf '  ✗ %s %s — not on crates.io\n' "$name" "$version"
           missing+=("crates:$name@$version") ;;
      *)   printf '  ? %s %s — crates.io returned %s\n' "$name" "$version" "$code"
           missing+=("crates:$name@$version (HTTP $code)") ;;
    esac
  done < <(cargo_publishable_crates)
}

# ── git tags ──────────────────────────────────────────────────────────────────
# Against the REMOTE. A tag in the runner's clone is the failure being tested
# for, not evidence against it.
verify_tags() {
  echo "── git tags (remote) ──"
  local remote_tags name version private tag legacy
  remote_tags=$(git ls-remote --tags origin | sed 's#.*refs/tags/##' | sed 's/\^{}$//' | sort -u)
  # Versions published before anything pushed tags. See the file's own header.
  legacy=$(grep -vE '^\s*(#|$)' "$LEGACY_FILE" 2>/dev/null || true)

  while IFS= read -r pkg; do
    json=$(committed "$pkg") || continue
    [ -z "$json" ] && continue
    IFS=$'\t' read -r name version private < <(json_package_fields <<<"$json")
    [ "$private" = "true" ] && continue
    tag="$name@$version"
    if grep -qxF "$tag" <<<"$remote_tags"; then
      # A tag on the legacy list that now exists means the list is stale, and a
      # stale exemption is how a gate quietly stops gating. Say so.
      if grep -qxF "$tag" <<<"$legacy"; then
        printf '  ✗ %s — exists, but is still listed in %s; remove that line\n' \
          "$tag" "$LEGACY_FILE"
        missing+=("stale-exemption:$tag")
      else
        printf '  ✓ %s\n' "$tag"
      fi
    elif grep -qxF "$tag" <<<"$legacy"; then
      printf '  ~ %s — untagged, known (predates tag pushing)\n' "$tag"
    else
      printf '  ✗ %s — no such tag on origin\n' "$tag"
      missing+=("tag:$tag")
    fi
  done < <(find packages -maxdepth 2 -name package.json -not -path '*/node_modules/*' | sort)
}

case "$section" in
  npm)    verify_npm ;;
  crates) verify_crates ;;
  tags)   verify_tags ;;
  all)    verify_npm; echo; verify_crates; echo; verify_tags ;;
  *) echo "unknown section '$section' — use npm, crates, tags, or all" >&2; exit 2 ;;
esac

echo
if [ ${#missing[@]} -eq 0 ]; then
  echo "✦ everything the repository declares is on the registry, and tagged."
  exit 0
fi

echo "✗ ${#missing[@]} thing(s) the repository declares are not actually published:" >&2
printf '    %s\n' "${missing[@]}" >&2
echo >&2
echo "A publish step exiting 0 is not evidence anything was published — that is" >&2
echo "what this script exists to say. Re-run the release workflow; if it reports" >&2
echo "success again while this still fails, the publish step is lying and the" >&2
echo "workflow is the bug." >&2
exit 1
