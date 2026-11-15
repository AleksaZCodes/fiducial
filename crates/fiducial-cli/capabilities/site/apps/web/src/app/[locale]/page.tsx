import { EntryCards } from "@/components/entry-cards";
import { metadataForPath } from "@/lib/metadata";
import { isLocale, localePath } from "@/lib/routes";
import { type SiteEntry, entryPath, latest } from "@/lib/site";
import { t } from "@/lib/t";
import { notFound } from "next/navigation";

type Props = { params: Promise<{ locale: string }> };

export async function generateMetadata({ params }: Props) {
  const { locale } = await params;
  return isLocale(locale) ? metadataForPath(localePath(locale, "/")) : {};
}

export default async function Home({ params }: Props) {
  const { locale } = await params;
  if (!isLocale(locale)) notFound();
  const entries = latest(locale);
  return (
    <div className="flex flex-col gap-12">
      <section className="flex flex-col gap-3">
        <h1 className="type-display">{t(locale, "meta.title")}</h1>
        <p className="type-subhead text-muted-foreground">{t(locale, "meta.description")}</p>
      </section>
      {entries.length > 0 ? (
        <section className="flex flex-col gap-6">
          <h2 className="type-h2">{t(locale, "site.latest")}</h2>
          <EntryCards
            entries={entries}
            locale={locale}
            // The cards hand back the entries they were given, so each knows
            // its collection, and two collections may share a slug.
            href={(e) => localePath(locale, entryPath((e as SiteEntry).collection, e.slug))}
            readMoreLabel={t(locale, "site.readMore")}
          />
        </section>
      ) : null}
    </div>
  );
}
