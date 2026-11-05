/**
 * The published package contains every file the CLI imports.
 *
 * `@fiducial/advisor@0.4.0` shipped without `src/review.js`: `files` in
 * `package.json` is an allowlist, a new module was imported by `cli.js`, and
 * nobody added it to the list. Every test ran from the repository, where the file
 * exists, so all of them passed while a clean install crashed at import on every
 * command, `status` included.
 *
 * This test packs the package the way `npm publish` would and checks the result
 * against what the code imports, so the allowlist cannot fall behind the source
 * again.
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

function packed() {
  const out = execFileSync("npm", ["pack", "--dry-run", "--json", "--ignore-scripts"], {
    cwd: root,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "ignore"],
  });
  return new Set(JSON.parse(out)[0].files.map((f) => f.path));
}

/** Relative `./x.js` imports of a source file. */
function relativeImports(file) {
  const text = readFileSync(join(root, file), "utf8");
  return [...text.matchAll(/from\s+["']\.\/([\w-]+\.js)["']/g)].map((m) => `src/${m[1]}`);
}

describe("published package", () => {
  it("includes every module the CLI reaches", () => {
    const files = packed();
    const seen = new Set();
    const queue = ["src/cli.js"];
    while (queue.length > 0) {
      const file = queue.pop();
      if (seen.has(file)) continue;
      seen.add(file);
      assert.ok(files.has(file), `${file} is imported but not in the published package: add it to "files"`);
      queue.push(...relativeImports(file));
    }
    assert.ok(seen.size > 1, "the walk should have followed imports");
  });

  it("includes every non-test module in src", () => {
    const files = packed();
    const modules = readdirSync(join(root, "src")).filter((f) => f.endsWith(".js") && !f.endsWith(".test.js"));
    for (const m of modules) {
      assert.ok(files.has(`src/${m}`), `src/${m} exists but is not in the published package`);
    }
  });

  it("leaves the tests out", () => {
    const files = packed();
    assert.ok(![...files].some((f) => f.endsWith(".test.js")), "test files must not be published");
  });
});
