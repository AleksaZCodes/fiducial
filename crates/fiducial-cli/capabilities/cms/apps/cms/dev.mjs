// `pnpm dev` in apps/cms — the editor, plus a watcher that re-derives.
//
// The site renders from `generated/*`, not from the files an editor writes, so
// a save without a derive shows nothing on the site and leaves the repository
// stale, which `fid derive --check` then fails on long after the person editing
// has gone. The watcher closes that gap.
//
// Everything the editor writes is an ordinary file change: review it with
// `git diff` and commit it. There is no store and no sync step.
import { spawn } from "node:child_process";
import { existsSync, watch } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../..", import.meta.url));
// `root`: Keystatic resolves content paths against the directory it runs in, so it
// runs from the repository root. `here`: Tina finds its config next to itself.
const [where, command, ...args] = process.argv.slice(2);
const shell = process.platform === "win32";

const editor = spawn(command, args, {
  cwd: where === "root" ? repo : fileURLToPath(new URL(".", import.meta.url)),
  stdio: "inherit",
  shell,
});
editor.on("exit", (code) => process.exit(code ?? 0));

// Debounced: one save writes several files, and a derive per file would report
// failures for half-written entries.
let timer;
let running = false;
const derive = () => {
  clearTimeout(timer);
  timer = setTimeout(() => {
    if (running) return;
    running = true;
    spawn("fid", ["derive"], { cwd: repo, stdio: "inherit", shell }).on("close", () => {
      running = false;
    });
  }, 600);
};
for (const path of ["content", "press", "messages", "settings.json"]) {
  const abs = join(repo, path);
  if (existsSync(abs)) watch(abs, { recursive: true }, derive);
}

for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => editor.kill());
