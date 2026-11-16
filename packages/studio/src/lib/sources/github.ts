import type { Schema, Source } from "../types.ts";

export interface GithubTarget {
  owner: string;
  repo: string;
  /** The branch Studio reads, and where a saved edit lands in `commit` mode. */
  branch: string;
  /** A token with Contents (and, for `pr`, Pull requests) write on this repository. */
  token: string;
  /**
   * `commit`: every save is a commit on `branch`.
   * `pr`: saves go to a new `studio/…` branch and `review.open` raises one pull
   * request for all of them, so nothing reaches `branch` unreviewed.
   */
  mode?: "commit" | "pr";
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
 * `fid` is involved: this is how a phone, or any browser, edits a product.
 *
 * It cannot run `fid derive` or a dev server (nothing here executes code), so
 * it leaves `dash`, `derive` and `dev` out and the UI hides what it cannot do.
 * `derive` happens where `fid` is: in CI, on the commit this makes.
 *
 * `fetchImpl` is injectable so the whole thing is tested without a network.
 */
export function githubSource(t: GithubTarget, schema: Schema, fetchImpl: typeof fetch = fetch): Source {
  const repo = `${t.owner}/${t.repo}`;
  const pr = t.mode === "pr";
  /** The `studio/…` branch edits go to, once one exists. */
  let work: string | null = null;

  async function gh<T>(method: string, path: string, body?: unknown): Promise<T> {
    const res = await fetchImpl(`${API}${path}`, {
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
  const reading = () => work ?? t.branch;

  /** In `pr` mode, make the work branch from the base branch's head, once. */
  async function ensureBranch(): Promise<string> {
    if (!pr) return t.branch;
    if (work) return work;
    const head = await gh<{ object: { sha: string } }>(
      "GET",
      `/repos/${repo}/git/ref/heads/${enc(t.branch)}`,
    );
    const name = `studio/${Date.now().toString(36)}`;
    await gh("POST", `/repos/${repo}/git/refs`, { ref: `refs/heads/${name}`, sha: head.object.sha });
    work = name;
    return name;
  }

  const source: Source = {
    kind: "github",
    label: `${repo}@${t.branch}`,
    async files() {
      const tree = await gh<{ tree: { path: string; type: string }[] }>(
        "GET",
        `/repos/${repo}/git/trees/${encodeURIComponent(reading())}?recursive=1`,
      );
      return tree.tree
        .filter((e) => e.type === "blob" && TEXT.test(e.path) && !HIDDEN.test(e.path) && !SECRET.test(e.path))
        .map((e) => e.path);
    },
    async read(path) {
      const f = await gh<{ content: string; sha: string }>(
        "GET",
        `/repos/${repo}/contents/${enc(path)}?ref=${encodeURIComponent(reading())}`,
      );
      shas.set(path, f.sha);
      return b64decode(f.content);
    },
    async write(path, text, message) {
      const branch = await ensureBranch();
      // The blob sha is the lock: GitHub refuses the commit if the file changed
      // since it was read, so two editors cannot silently overwrite each other.
      const sha = shas.get(path);
      const r = await gh<{ content: { sha: string } }>("PUT", `/repos/${repo}/contents/${enc(path)}`, {
        message: message ?? `studio: edit ${path}`,
        content: b64encode(text),
        branch,
        ...(sha ? { sha } : {}),
      });
      shas.set(path, r.content.sha);
    },
    // No `fid` runs here, so the schema ships inside the app. It is generated
    // by `fid studio schema` and a test fails if the committed copy drifts.
    schema: async () => schema,
  };

  if (pr) {
    source.review = {
      branch: () => work,
      async open(title, body) {
        if (!work) throw new Error("nothing saved yet — there is no branch to propose");
        const made = await gh<{ html_url: string }>("POST", `/repos/${repo}/pulls`, {
          title,
          head: work,
          base: t.branch,
          body: body ?? "Edited in Studio.",
        });
        return made.html_url;
      },
    };
  }
  return source;
}
