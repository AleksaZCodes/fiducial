import { type CollectionName, content } from "@/generated/content";
import type { Locale } from "@/generated/messages";

/**
 * The content collections as pages: which ones have a URL, what an entry's URL
 * is, and the newest entries across them. Framework-free, so the SvelteKit
 * shape can use it unchanged.
 *
 * A collection is a page source when `content.toml` gives it a `route` with
 * `{slug}` in it. The route is declared once there, and the sitemap, the
 * metadata and these pages all read the same declaration.
 */
export type SiteEntry = {
  collection: CollectionName;
  slug: string;
  title: string;
  date?: string;
  summary?: string;
  tags?: readonly string[];
  body?: string;
};

const routeOf = (name: CollectionName): string => content.collections[name].route as string;

export function routedCollections(): CollectionName[] {
  return (Object.keys(content.collections) as CollectionName[]).filter((name) =>
    routeOf(name).includes("{slug}"),
  );
}

/** The path of an entry, without the locale prefix: `/blog/hello`. */
export function entryPath(collection: CollectionName, slug: string): string {
  return routeOf(collection).replace("{slug}", slug);
}

export function entriesOf(collection: CollectionName, locale: Locale): SiteEntry[] {
  const list = content.collections[collection].byLocale[locale] as unknown as readonly Record<
    string,
    unknown
  >[];
  return list.map((e) => ({
    ...(e as Omit<SiteEntry, "collection" | "title">),
    collection,
    // An entry is named by its first text field, as the editor names it.
    title: String(e.title ?? e.name ?? e.headline ?? e.slug),
  }));
}

/** Every routed entry in a locale, newest first; entries without a date last. */
export function latest(locale: Locale): SiteEntry[] {
  return routedCollections()
    .flatMap((c) => entriesOf(c, locale))
    .sort((a, b) => (b.date ?? "").localeCompare(a.date ?? ""));
}

/** The entry at a path (`/blog/hello`), if any collection serves it. */
export function entryAt(locale: Locale, path: string): SiteEntry | undefined {
  return latest(locale).find((e) => entryPath(e.collection, e.slug) === path);
}
