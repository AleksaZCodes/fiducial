#!/usr/bin/env bash
#
# One GitHub release per platform version: `vX.Y.Z`, the workspace version.
#
#   scripts/github-release.sh            # create v<workspace version> if missing
#   scripts/github-release.sh --dry-run  # print the notes, create nothing
#
# Why one, not one per package: the platform ships as fifteen crates and a
# dozen npm packages, and a reader — or Zenodo, which mints a DOI for every
# GitHub release it is told about — needs one answer to "which Fiducial is
# this?". Per-package releases gave six of the npm packages a release, the
# rest only a tag, and the crates nothing: an accident of which publish runs
# happened to succeed, not a decision. Per-package *tags* stay — changesets
# uses them — but the release is the platform's.
#
# The notes list exactly what this version is: the workspace version every
# crate carries, and the version of each public npm package, read from the
# repository — not typed.
#
# Idempotent: when the release exists it does nothing. Needs `gh` and
# GH_TOKEN (or GITHUB_TOKEN) when not a dry run.
set -euo pipefail
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -1)
[ -n "$version" ] || { echo "no [workspace.package] version in Cargo.toml" >&2; exit 1; }
tag="v$version"

notes=$(mktemp)
{
  echo "Fiducial $version — the platform at this version."
  echo
  echo "Install the CLI: \`cargo install fiducial-cli --version $version\`."
  echo
  echo "## Rust crates (crates.io) — all at $version"
  echo
  for m in crates/*/Cargo.toml; do
    name=$(sed -n 's/^name *= *"\([^"]*\)".*/\1/p' "$m" | head -1)
    if grep -q '^publish *= *false' "$m"; then continue; fi
    echo "- [\`$name\`](https://crates.io/crates/$name/$version)"
  done
  echo
  echo "## npm packages"
  echo
  for p in packages/*/package.json; do
    node -e '
      const p = require("./" + process.argv[1]);
      if (!p.private) console.log(`- [\`${p.name}@${p.version}\`](https://www.npmjs.com/package/${p.name}/v/${p.version})`);
    ' "$p"
  done
  echo
  echo "What changed: \`SHIPPED.md\` and each package's \`CHANGELOG.md\`."
} > "$notes"

if [ "${1:-}" = "--dry-run" ]; then
  echo "would create $tag:"
  cat "$notes"
  exit 0
fi

if gh release view "$tag" >/dev/null 2>&1; then
  echo "$tag already released — nothing to do"
  exit 0
fi

gh release create "$tag" --title "Fiducial $version" --notes-file "$notes" --target "$(git rev-parse HEAD)"
echo "released $tag"
