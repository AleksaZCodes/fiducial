import type { Schema, Source } from "../types.ts";

export interface GithubTarget {
  owner: string;
  repo: string;
  /** The branch Studio reads and commits to. */
  branch: string;
  /** A fine-grained token with Contents: read/write on this repository. */
  token: string;
}

const API = "https://api.github.com";

/** Text a workbench edits: the same list `fid studio` allows. */
const TEXT = /\.(toml|md|mdx|json|css|ts|tsx|js|jsx|mjs|svelte|astro|html|ya?ml|sql|rs|txt)$/i;
const HIDDEN = /(^|\/)(node_modules|target|dist|\.next|\.turbo|\.wrangler|\.git)\//;
const SECRET = /(^|\/)(\.env(\..*)?|\.dev\.vars|[^/]*\.(pem|key))$/;

export function b64encode(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}

export function b64decode(b64: string): string {
  const bin = atob(b64.replace(/\n/g, ""));
  return new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
}

/**
 * The source backed by a repository branch, through GitHub's REST API. No
 * `fid` is involved: this is how a phone, or any browser, edits a product. A
 * save is a commit on `branch`, so history, review and rollback are Git's.
 *
 * It cannot run `fid derive` or a dev server (nothing here executes code), so
 * it leaves `dash`, `derive` and `dev` out and the UI hides what it cannot do.
 * `derive` happens where `fid` is: in CI, on the commit this makes.
 */
export function githubSource(t: GithubTarget, schema: Schema): Source {
  const repo = `${t.owner}/${t.repo}`;
  async function gh<T>(method: string, path: string, body?: unknown): Promise<T> {
    const res = await fetch(`${API}${path}`, {
      method,
      body: body === undefined ? undefined : JSON.stringify(body),
      headers: {
        Authorization: `Bearer ${t.token}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
      },
    });
    const data = (await res.json().catch(() => ({}))) as T & { message?: string };
    if (!res.ok) throw new Error(data.message ?? `GitHub ${res.status}`);
    return data;
  }
  const enc = (p: string) => p.split("/").map(encodeURIComponent).join("/");
  const shas = new Map<string, string>();

  return {
    kind: "github",
    label: `${repo}@${t.branch}`,
    async files() {
      const tree = await gh<{ tree: { path: string; type: string }[] }>(
        "GET",
        `/repos/${repo}/git/trees/${encodeURIComponent(t.branch)}?recursive=1`,
      );
      return tree.tree
        .filter((e) => e.type === "blob" && TEXT.test(e.path) && !HIDDEN.test(e.path) && !SECRET.test(e.path))
        .map((e) => e.path);
    },
    async read(path) {
      const f = await gh<{ content: string; sha: string }>(
        "GET",
        `/repos/${repo}/contents/${enc(path)}?ref=${encodeURIComponent(t.branch)}`,
      );
      shas.set(path, f.sha);
      return b64decode(f.content);
    },
    async write(path, text, message) {
      // The blob sha is the lock: GitHub refuses the commit if the file changed
      // since it was read, so two editors cannot silently overwrite each other.
      const sha = shas.get(path);
      const r = await gh<{ content: { sha: string } }>("PUT", `/repos/${repo}/contents/${enc(path)}`, {
        message: message ?? `studio: edit ${path}`,
        content: b64encode(text),
        branch: t.branch,
        ...(sha ? { sha } : {}),
      });
      shas.set(path, r.content.sha);
    },
    // No `fid` runs here, so the schema ships inside the app. It is generated
    // by `fid studio schema` and a test fails if the committed copy drifts.
    schema: async () => schema,
  };
}
