import { locales } from "@/generated/messages";
import { pressFor } from "@/generated/press";
import { formatContentDate } from "@/lib/intl";
import { renderBody } from "@/lib/markdown";
import { mediaUrl } from "@/lib/media";
import { metadataForPath } from "@/lib/metadata";
import { isLocale, localePath } from "@/lib/routes";
import { notFound } from "next/navigation";

export const dynamicParams = false;

export function generateStaticParams() {
  return locales.flatMap((locale) =>
    pressFor(locale).stories.map(({ slug }) => ({ locale, slug })),
  );
}

type Props = { params: Promise<{ locale: string; slug: string }> };

export async function generateMetadata({ params }: Props) {
  const { locale, slug } = await params;
  return isLocale(locale) ? metadataForPath(localePath(locale, `/press/${slug}`)) : {};
}

export default async function Story({ params }: Props) {
  const { locale, slug } = await params;
  if (!isLocale(locale)) notFound();
  const story = pressFor(locale).stories.find((s) => s.slug === slug);
  if (!story) notFound();
  return (
    <article className="flex flex-col gap-6">
      <header className="flex flex-col gap-2">
        <time className="type-small text-muted-foreground" dateTime={story.date}>
          {formatContentDate(story.date, locale)}
        </time>
        <h1 className="type-h1">{story.title}</h1>
        <p className="type-subhead text-muted-foreground">{story.summary}</p>
      </header>
      {story.cover ? (
        <img src={mediaUrl(story.cover)} alt={story.coverAlt} className="w-full" />
      ) : null}
      <div
        className="type-body flex flex-col gap-4"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: rendered from our own Markdown, HTML escaped
        dangerouslySetInnerHTML={{ __html: renderBody(story.body) }}
      />
    </article>
  );
}
