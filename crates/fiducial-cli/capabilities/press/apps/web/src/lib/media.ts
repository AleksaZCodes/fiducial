/**
 * Where a piece of media lives, by key.
 *
 * Content never stores a URL. A blog post, a press story or a gallery entry
 * names an object key — `press/stories/cry-wolf.png` — and this turns it into
 * something a browser can fetch. That is the whole of the coupling between the
 * content and the storage vendor, which is what keeps `[adapters] storage`
 * swappable: moving from R2 to anything else changes how this path is served,
 * not a single content file.
 *
 * Plain function, no framework import, so a SvelteKit port reuses it as is.
 */
export const MEDIA_BASE = "/media";

export function mediaUrl(key: string): string {
  return `${MEDIA_BASE}/${key.replace(/^\/+/, "")}`;
}
