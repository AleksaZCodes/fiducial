import type { Json } from "./toml.ts";
import { getPath } from "./toml.ts";

/**
 * Where a person goes to edit content, given how the product is set up and how
 * Studio is connected. Pure, so every case is a test.
 *
 * Three editors read the same files (`[cms] editor`), and each is reached
 * differently:
 *
 * | editor | local | hosted |
 * |---|---|---|
 * | `pages` | n/a | `app.pagescms.org/<owner>/<repo>/<branch>`, signs in with GitHub |
 * | `keystatic` | `apps/cms` dev server, `/keystatic` | the deployed site's `/keystatic` when `storage = "github"` |
 * | `tina` | `apps/cms` dev server, `/admin` | Tina Cloud (its own login) |
 */
export type EditorLink =
  | { kind: "open"; label: string; url: string }
  | { kind: "start"; label: string; path: string }
  | { kind: "none"; reason: string };

export interface EditorContext {
  /** Parsed `fiducial.toml`. */
  config: Record<string, Json>;
  /** Set when Studio is connected to a GitHub repository. */
  github?: { owner: string; repo: string; branch: string };
  /** The editor dev server's URL, when it is running locally. */
  devUrl?: string | null;
  /** Whether Studio can start a local dev server at all. */
  canStartDev: boolean;
}

export function editorLink(c: EditorContext): EditorLink {
  const cms = getPath(c.config, ["cms"]);
  if (cms === undefined || typeof cms !== "object" || cms === null || Array.isArray(cms)) {
    return { kind: "none", reason: "This product has no content editor (the `cms` capability)." };
  }
  const editor = typeof cms.editor === "string" ? cms.editor : "keystatic";
  const domain = getPath(c.config, ["brand", "domain"]);
  const site = typeof domain === "string" && domain ? `https://${domain.replace(/^https?:\/\//, "").replace(/\/$/, "")}` : null;

  if (editor === "pages") {
    const repo = c.github ? `${c.github.owner}/${c.github.repo}` : typeof cms.repo === "string" ? cms.repo : null;
    if (!repo) return { kind: "none", reason: "Pages CMS needs a repository: connect through GitHub, or set `[cms] repo`." };
    const branch = c.github?.branch ?? "main";
    return { kind: "open", label: "Open in Pages CMS", url: `https://app.pagescms.org/${repo}/${branch}` };
  }

  if (editor === "keystatic" && cms.storage === "github") {
    return site
      ? { kind: "open", label: "Open Keystatic", url: `${site}/keystatic` }
      : { kind: "none", reason: "Hosted Keystatic lives on the deployed site: set `[brand] domain`." };
  }

  if (editor === "tina" && c.github) {
    return { kind: "open", label: "Open Tina Cloud", url: "https://app.tina.io" };
  }

  // Local editors: Keystatic writing to the working tree, or Tina's dev server.
  const path = editor === "tina" ? "admin" : "keystatic";
  if (c.devUrl) {
    return { kind: "open", label: `Open ${editor === "tina" ? "Tina" : "Keystatic"}`, url: `${c.devUrl.replace(/\/$/, "")}/${path}` };
  }
  if (c.canStartDev) return { kind: "start", label: `Start ${editor === "tina" ? "Tina" : "Keystatic"}`, path };
  return { kind: "none", reason: "A local editor needs `fid studio` on a machine with the product checked out." };
}

/** The deployed site, as a preview target for a source that cannot run one. */
export function deployedUrl(config: Record<string, Json>): string | null {
  const domain = getPath(config, ["brand", "domain"]);
  if (typeof domain === "string" && domain) return `https://${domain.replace(/^https?:\/\//, "").replace(/\/$/, "")}`;
  const route = getPath(config, ["deploy", "route"]);
  if (typeof route === "string" && route) return `https://${route.replace(/\/\*$/, "").replace(/^https?:\/\//, "")}`;
  return null;
}
