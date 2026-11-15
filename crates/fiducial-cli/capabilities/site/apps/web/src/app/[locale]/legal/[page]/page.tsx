import { LegalDocument } from "@/components/legal-document";
import { type LegalPage, legalCatalogs } from "@/generated/legal";
import { locales } from "@/generated/messages";
import { metadataForPath } from "@/lib/metadata";
import { isLocale, localePath } from "@/lib/routes";
import { notFound } from "next/navigation";

// One page per page the jurisdiction produced, per locale: read from the
// catalog, so a page the jurisdiction adds is routed without an edit here.
export const dynamicParams = false;

export function generateStaticParams() {
  return locales.flatMap((locale) =>
    Object.keys(legalCatalogs[locale] ?? {}).map((page) => ({ locale, page })),
  );
}

type Props = { params: Promise<{ locale: string; page: string }> };

export async function generateMetadata({ params }: Props) {
  const { locale, page } = await params;
  return isLocale(locale) ? metadataForPath(localePath(locale, `/legal/${page}`)) : {};
}

export default async function LegalRoute({ params }: Props) {
  const { locale, page } = await params;
  const doc = isLocale(locale) ? legalCatalogs[locale]?.[page as LegalPage] : undefined;
  if (!doc) notFound();
  return <LegalDocument title={doc.title} body={doc.body} />;
}
