# Reading JSON in the release scripts, without jq.
#
# These three scripts used `jq` for two questions and nothing else: what a
# `package.json` declares, and which workspace crates are publishable. `jq` is
# preinstalled on GitHub's ubuntu runners, so that dependency was invisible —
# until `scripts/backfill-tags.sh` was run on a maintainer's machine, where it
# died on `jq: command not found` after printing nothing.
#
# A prerequisite that only CI happens to satisfy is not a prerequisite anyone
# declared; it is one the runner image donated. The fix is not to document it
# or vendor a per-platform binary, but to stop needing it: this repository
# already requires Node on every machine that touches it (`.nvmrc`, `engines`,
# `packageManager`), so `node -e` is a dependency that is *declared*.
#
# Source it with, after the `repo` assignment each script already makes:
#
#   . "$repo/scripts/lib/json.sh"

# Emit `name<TAB>version<TAB>private` for one package.json read from stdin.
#
# One process for three fields, where the jq version spawned three. `private`
# is normalised to `true`/`false` so the caller compares a string and never
# has to think about the key being absent.
json_package_fields() {
  node -e '
    const p = JSON.parse(require("fs").readFileSync(0, "utf8"));
    process.stdout.write([p.name, p.version, p.private === true].join("\t") + "\n");
  '
}

# Emit `name<TAB>version` for every publishable workspace crate, sorted.
#
# "Publishable" is Cargo's own rule and not ours: `publish = false` serialises
# as `[]` in `cargo metadata`, an explicit registry list as a non-empty array,
# and the default as `null`. Only the empty array means "do not publish" —
# which is what the `.publish != []` this replaces was saying.
cargo_publishable_crates() {
  cargo metadata --no-deps --format-version 1 | node -e '
    const m = JSON.parse(require("fs").readFileSync(0, "utf8"));
    for (const p of m.packages) {
      if (Array.isArray(p.publish) && p.publish.length === 0) continue;
      process.stdout.write(p.name + "\t" + p.version + "\n");
    }
  ' | sort
}
