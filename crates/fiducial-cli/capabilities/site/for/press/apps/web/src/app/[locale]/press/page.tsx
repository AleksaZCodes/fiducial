import { PressKit } from "@/components/press-kit";
import type { MessageKey } from "@/generated/messages";
import { formatContentDate } from "@/lib/intl";
import { metadataForPath } from "@/lib/metadata";
import { isLocale, localePath } from "@/lib/routes";
import { t } from "@/lib/t";
import { notFound } from "next/navigation";

type Props = { params: Promise<{ locale: string }> };

export async function generateMetadata({ params }: Props) {
  const { locale } = await params;
  return isLocale(locale) ? metadataForPath(localePath(locale, "/press")) : {};
}

const LABELS = [
  "title",
  "assets",
  "guidelines",
  "facts",
  "boilerplate",
  "short",
  "medium",
  "long",
  "stories",
  "contact",
  "download",
] as const;

export default async function Press({ params }: Props) {
  const { locale } = await params;
  if (!isLocale(locale)) notFound();
  const labels = Object.fromEntries(
    LABELS.map((k) => [k, t(locale, `press.${k}` as MessageKey)]),
  ) as Record<(typeof LABELS)[number], string>;
  return (
    <PressKit
      locale={locale}
      labels={labels}
      // The brand primitive, served from the repository: `brand` derives it.
      logos={[{ label: t(locale, "meta.title"), href: "/favicon.svg" }]}
      gallery={[]}
      storyHref={(slug) => localePath(locale, `/press/${slug}`)}
      angleLabel={(angle) => t(locale, `press.angle.${angle}` as MessageKey)}
      formatDate={(iso) => formatContentDate(iso, locale)}
    />
  );
}
