#!/usr/bin/env bash
#
# Every vendor skill a capability names can still be installed, the way a product
# installs it.
#
# A capability names skills it does not own (Cloudflare's, shadcn's). If a vendor
# renames or moves one, a product is told to load a skill that is not there. This
# does what a product does and asks nothing of anyone's pages: scaffold a product
# with every capability that names a skill, run `fid skills sync --install` (which
# runs the open `skills` CLI), check each named skill landed in the shared
# `.agents/skills/` directory, that it is linked for the agents the product names,
# and that `skills check` finds the lock consistent with upstream.
#
# It runs weekly as well as on changes, because a vendor can break this without a
# commit here; the failure is then a one-line edit to a `capability.toml`, found
# this week.
#
# Needs Node 22.20+ and the network.
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-agent-skills.sh
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "── agent skills ───────────────────────────────────────────────────────"
list="$work/declared.json"
"$fid" skills list --all > "$list"

# The capabilities that name a skill, and every skill name they name.
caps="$(node -e 'const d=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")); console.log([...new Set(d.flatMap(x=>x.capabilities))].join(" "))' "$list")"
names="$(node -e 'const d=JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")); console.log([...new Set(d.flatMap(x=>x.skills))].join(" "))' "$list")"
[ -n "$caps" ] && [ -n "$names" ] || { echo "no capability names an agent skill" >&2; exit 1; }
echo "  capabilities: $caps"
echo "  skills:       $names"

cd "$work"
"$fid" new p >/dev/null
cd p
"$fid" add design >/dev/null
for c in $caps; do
  [ "$c" = design ] && continue
  "$fid" add capability "$c" >/dev/null 2>&1 || "$fid" add app "${c#web-}" >/dev/null 2>&1 || {
    echo "  ✗ could not install capability $c into a scaffolded product" >&2
    exit 1
  }
done

# The production workflow: the CLI does the install and writes the lock.
"$fid" skills sync --install

bad=0
for n in $names; do
  if [ -f ".agents/skills/$n/SKILL.md" ] && grep -q "\"$n\"" skills-lock.json; then
    echo "  ✓ $n"
  else
    echo "  ✗ $n: not installed, or not in skills-lock.json" >&2
    bad=1
  fi
done
# Claude Code is in the default agents, so each is linked into its directory too.
for n in $names; do
  [ -f ".claude/skills/$n/SKILL.md" ] || { echo "  ✗ $n: not linked for claude-code" >&2; bad=1; }
done
[ "$bad" = 0 ] || exit 1

# The lock agrees with upstream, and nothing the product needs is missing.
"$fid" skills check
echo "agent-skills: ok"
