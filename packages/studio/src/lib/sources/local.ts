import type { DevStatus, Dash, DeriveResult, Schema, Source } from "../types.ts";

/**
 * The source backed by `fid studio` on this machine. Files are on disk; saving
 * writes them there. `base` is "" when Studio is served by `fid studio` itself
 * (same origin), or the server's URL during development.
 */
export function localSource(base: string, token: string): Source {
  async function call<T>(method: string, path: string, body?: string): Promise<T> {
    const res = await fetch(`${base}${path}`, {
      method,
      body,
      headers: { Authorization: `Bearer ${token}` },
    });
    const data = (await res.json().catch(() => ({}))) as T & { error?: string };
    if (!res.ok) throw new Error(data.error ?? `${res.status} ${res.statusText}`);
    return data;
  }
  const q = (p: string) => encodeURIComponent(p);
  const dev = (action: string, method = "POST") => call<DevStatus>(method, `/api/dev${action}`);

  return {
    kind: "local",
    label: base || "this machine",
    files: async () => (await call<{ files: string[] }>("GET", "/api/files")).files,
    read: async (path) => (await call<{ text: string }>("GET", `/api/file?path=${q(path)}`)).text,
    write: async (path, text) => {
      await call("PUT", `/api/file?path=${q(path)}`, text);
    },
    schema: () => call<Schema>("GET", "/api/schema"),
    dash: () => call<Dash>("GET", "/api/dash"),
    derive: (check) => call<DeriveResult>("POST", `/api/derive${check ? "?check=1" : ""}`),
    dev: {
      status: () => dev("", "GET"),
      start: () => dev("/start"),
      stop: () => dev("/stop"),
    },
  };
}
