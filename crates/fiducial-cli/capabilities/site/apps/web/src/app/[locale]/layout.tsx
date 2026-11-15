import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { domain } from "@/generated/brand";
import { fontVariables } from "@/generated/fonts";
import { locales, messages } from "@/generated/messages";
import { isLocale } from "@/lib/routes";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import "../globals.css";

// A locale that is not declared is a 404, not a page rendered in a fallback.
export const dynamicParams = false;

export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

export const metadata: Metadata = { metadataBase: new URL(`https://${domain}`) };

export default async function LocaleLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  if (!isLocale(locale)) notFound();
  return (
    <html lang={messages[locale]["locale.tag"]} className={fontVariables}>
      <body className="flex min-h-dvh flex-col bg-background text-foreground">
        <SiteHeader locale={locale} />
        <main className="mx-auto w-full max-w-3xl flex-1 px-6 py-12">{children}</main>
        <SiteFooter locale={locale} />
      </body>
    </html>
  );
}
