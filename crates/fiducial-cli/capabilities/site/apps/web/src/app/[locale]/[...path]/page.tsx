import { locales } from "@/generated/messages";
import { formatContentDate } from "@/lib/intl";
import { renderBody } from "@/lib/markdown";
import { metadataForPath } from "@/lib/metadata";
import { isLocale, localePath } from "@/lib/routes";
import { entriesOf, entryAt, entryPath, routedCollections } from "@/lib/site";
import { notFound } from "next/navigation";

/**
 * Every entry of every collection that declared a `route`, and nothing else:
 * a path no entry has is the 404, in the reader's language. The routes come
 * from `content.toml`, so a new collection with a route has pages on the next
 * build, and the sitemap already lists them.
 */
export const dynamicParams = false;

export function generateStaticParams() {
  return locales.flatMap((locale) =>
    routedCollections().flatMap((c) =>
      entriesOf(c, locale).map((e) => ({
        locale,
        path: entryPath(c, e.slug).split("/").filter(Boolean),
      })),
    ),
  );
}

type Props = { params: Promise<{ locale: string; path: string[] }> };

export async function generateMetadata({ params }: Props) {
  const { locale, path } = await params;
  return isLocale(locale) ? metadataForPath(localePath(locale, `/${path.join("/")}`)) : {};
}

export default async function EntryPage({ params }: Props) {
  const { locale, path } = await params;
  if (!isLocale(locale)) notFound();
  const entry = entryAt(locale, `/${path.join("/")}`);
  if (!entry) notFound();
  return (
    <article className="flex flex-col gap-6">
      <header className="flex flex-col gap-2">
        {entry.date ? (
          <time className="type-small text-muted-foreground" dateTime={entry.date}>
            {formatContentDate(entry.date, locale)}
          </time>
        ) : null}
        <h1 className="type-h1">{entry.title}</h1>
        {entry.summary ? (
          <p className="type-subhead text-muted-foreground">{entry.summary}</p>
        ) : null}
      </header>
      {entry.body ? (
        <div
          className="prose type-body flex flex-col gap-4"
          // Escaped by `renderBody`: raw HTML in an entry is printed as text.
          // biome-ignore lint/security/noDangerouslySetInnerHtml: rendered from our own Markdown, HTML escaped
          dangerouslySetInnerHTML={{ __html: renderBody(entry.body) }}
        />
      ) : null}
    </article>
  );
}
