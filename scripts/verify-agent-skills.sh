#!/usr/bin/env bash
#
# Every vendor skill a capability names still exists where the capability says
# it does.
#
# A capability names a skill it does not own (Cloudflare's, shadcn's). If the
# vendor renames or moves it, a product's agent is told to load a skill that is
# not there and the gate then blocks edits with no way through. This asks each
# upstream, and fails with the capability's own declaration beside what it found,
# so the fix is a one-line edit to a `capability.toml` rather than a mystery in a
# product months later. It runs on a schedule as well as on changes, because the
# vendor can break us without a commit here.
#
# For a plugin it reads the marketplace's `.claude-plugin/marketplace.json`,
# checks the plugin is listed, and checks each `plugin:skill` name has a
# `skills/<skill>/SKILL.md` in the plugin's source. For a vendored skill it checks
# the `SKILL.md` at the path the `skills` CLI installs from.
#
# Usage:
#
#   cargo build -p fiducial-cli --bin fid
#   scripts/verify-agent-skills.sh
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fid="${FID:-$repo/target/debug/fid}"
[ -x "$fid" ] || { echo "no fid binary at $fid — cargo build -p fiducial-cli --bin fid" >&2; exit 1; }

echo "── agent skills ───────────────────────────────────────────────────────"
"$fid" skills list --all | node --input-type=module -e '
import { readFileSync } from "node:fs";
const declared = JSON.parse(readFileSync(0, "utf8"));
if (declared.length === 0) { console.error("no capability declares an agent skill"); process.exit(1); }

const raw = (repo, path) => `https://raw.githubusercontent.com/${repo}/main/${path}`;
async function get(url) {
  const r = await fetch(url);
  return r.ok ? await r.text() : null;
}
let bad = 0;
const fail = (id, msg) => { console.error(`  ✗ ${id}: ${msg}`); bad++; };

for (const a of declared) {
  if (a.kind === "plugin") {
    const text = await get(raw(a.repo, ".claude-plugin/marketplace.json"));
    if (text === null) { fail(a.id, `${a.repo} has no .claude-plugin/marketplace.json on main`); continue; }
    const market = JSON.parse(text);
    if (market.name !== a.marketplace) {
      fail(a.id, `the marketplace is named "${market.name}", the capability says "${a.marketplace}"`);
      continue;
    }
    const plugin = (market.plugins ?? []).find((p) => p.name === a.plugin);
    if (!plugin) {
      fail(a.id, `no plugin "${a.plugin}" in ${a.repo} (it lists: ${(market.plugins ?? []).map((p) => p.name).join(", ")})`);
      continue;
    }
    if (plugin.source !== "./") {
      fail(a.id, `the plugin source is ${JSON.stringify(plugin.source)}; this check only knows "./" (extend it, do not skip it)`);
      continue;
    }
    for (const name of a.skills) {
      const dir = name.slice(a.plugin.length + 1);
      const md = await get(raw(a.repo, `skills/${dir}/SKILL.md`));
      if (md === null) fail(a.id, `${name}: no skills/${dir}/SKILL.md in ${a.repo}`);
      else if (!new RegExp(`^name:\\s*${dir}\\s*$`, "m").test(md)) fail(a.id, `${name}: skills/${dir}/SKILL.md does not declare name: ${dir}`);
    }
  } else {
    const md = await get(raw(a.repo, `skills/${a.skill}/SKILL.md`));
    if (md === null) fail(a.id, `no skills/${a.skill}/SKILL.md in ${a.repo}; \`skills add ${a.source}\` would not find it`);
    else if (!new RegExp(`^name:\\s*${a.skill}\\s*$`, "m").test(md)) fail(a.id, `skills/${a.skill}/SKILL.md does not declare name: ${a.skill}`);
  }
  if (!bad) console.log(`  ✓ ${a.id} (${a.kind}, ${a.repo})`);
}
if (bad) { console.error(`agent-skills: ${bad} problem(s)`); process.exit(1); }
console.log("agent-skills: ok");
'
