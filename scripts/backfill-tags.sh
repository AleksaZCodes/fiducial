#!/usr/bin/env bash
#
# Create the release tags that were never pushed, at the commits that made them.
#
# Why this exists: `changeset publish` printed "Creating git tags… Created git
# tags." on every release while nothing ever pushed them, so
# `git ls-remote --tags origin` was empty for twelve published versions. The
# workflow now pushes tags going forward (`release.yml`), but a fix cannot
# retroactively create what was dropped — hence this, once.
#
# Each tag points at the commit that introduced its version, found by searching
# that package.json's history for the version string. For most that is the
# `chore: version packages` commit changesets itself would have tagged; for a
# package first added at its current version it is the feature commit.
#
# Run it with credentials that may push tags:
#
#   scripts/backfill-tags.sh            # show what would be created
#   scripts/backfill-tags.sh --push     # create and push them
#
# `--push` also clears the tag names out of `docs/release/legacy-untagged.txt`,
# because leaving that to a human is leaving it undone: `verify-published.sh`
# FAILS on a listed tag that now exists, so the bookkeeping is not optional —
# it is the difference between a green build and a red one. The file's
# explanatory header is kept; only the names go.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo"

push=""
[ "${1:-}" = "--push" ] && push=1

created=()
while IFS= read -r pkg; do
  json=$(git show "HEAD:$pkg")
  [ "$(jq -r '.private // false' <<<"$json")" = "true" ] && continue
  name=$(jq -r '.name' <<<"$json")
  version=$(jq -r '.version' <<<"$json")
  tag="$name@$version"

  if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    echo "  = $tag already exists locally"
    continue
  fi

  # The commit that introduced this exact version string in this file.
  commit=$(git log HEAD --format=%H -1 -S"\"version\": \"$version\"" -- "$pkg")
  if [ -z "$commit" ]; then
    echo "  ? $tag — could not find the commit that set this version" >&2
    continue
  fi

  echo "  + $tag -> ${commit:0:8}  $(git log -1 --format=%s "$commit" | cut -c1-48)"
  if [ -n "$push" ]; then
    git tag "$tag" "$commit"
    created+=("$tag")
  fi
done < <(git ls-tree -r --name-only HEAD packages | grep '/package.json$' | sort)

if [ -z "$push" ]; then
  echo
  echo "(dry run — re-run with --push to create and push these)"
  exit 0
fi

if [ ${#created[@]} -eq 0 ]; then
  echo "✦ nothing to create"
  exit 0
fi

git push origin "${created[@]/#/refs/tags/}"
echo "✦ pushed ${#created[@]} tag(s)"

# Drop the names we just created from the legacy list, keeping its header.
legacy="docs/release/legacy-untagged.txt"
if [ -f "$legacy" ]; then
  tmp=$(mktemp)
  while IFS= read -r line; do
    case "$line" in
      ''|\#*) echo "$line" ;;
      *) printf '%s\n' "${created[@]}" | grep -qxF "$line" || echo "$line" ;;
    esac
  done < "$legacy" > "$tmp"
  mv "$tmp" "$legacy"
  left=$(grep -cvE '^\s*(#|$)' "$legacy" || true)
  echo "✦ $legacy now lists $left untagged version(s)"
  echo
  echo "Commit that change — the list is part of the repository:"
  echo "    git add $legacy && git commit -m 'chore: the legacy tags exist now'"
fi

echo
echo "Verify with: scripts/verify-published.sh tags"
