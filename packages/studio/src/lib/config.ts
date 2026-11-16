/**
 * Deployment settings for this copy of Studio, read at runtime from
 * `studio.config.json` beside the app, so one build serves any deployment.
 *
 * - `authUrl`: the sign-in Worker (`worker/`). Empty means there is none, and
 *   the person pastes a token instead.
 */
export interface StudioConfig {
  authUrl: string;
}

export async function loadConfig(fetchImpl: typeof fetch = fetch): Promise<StudioConfig> {
  try {
    const res = await fetchImpl("./studio.config.json", { cache: "no-store" });
    if (!res.ok) return { authUrl: "" };
    const raw = (await res.json()) as Partial<StudioConfig>;
    return { authUrl: typeof raw.authUrl === "string" ? raw.authUrl.replace(/\/$/, "") : "" };
  } catch {
    return { authUrl: "" };
  }
}

/** The URL that starts GitHub sign-in and returns here. */
export function signInUrl(authUrl: string, here: string): string {
  return `${authUrl}/login?return=${encodeURIComponent(here)}`;
}
