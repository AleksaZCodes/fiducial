import type { Locale } from "@/generated/messages";
import { seoRoutes } from "@/generated/seo";
import { t } from "@/lib/t";
import Link from "next/link";

/**
 * Links to every page that is not an entry — the legal pages, the press room —
 * read from the route table rather than listed here, so a page a jurisdiction
 * adds is linked the moment it exists, in every language.
 */
export function SiteFooter({ locale }: { locale: Locale }) {
  // `string`, not the table's literal keys: without `press` there is no
  // "page/press" among them, and comparing to it would not type-check.
  const isLinked = (key: string) => key.startsWith("legal/") || key === "page/press";
  const pages = seoRoutes.filter((r) => r.locale === locale && isLinked(r.key));
  return (
    <footer className="border-t border-border">
      <div className="mx-auto flex w-full max-w-3xl flex-wrap items-center justify-between gap-4 px-6 py-6">
        <nav aria-label={t(locale, "site.legal")} className="flex flex-wrap gap-x-4 gap-y-1">
          {pages.map((r) => (
            <Link key={r.path} href={r.path} className="type-small text-muted-foreground">
              {r.title}
            </Link>
          ))}
        </nav>
        <p className="type-small text-muted-foreground">
          {t(locale, "footer.madeWith")}{" "}
          <a href="https://github.com/AleksaZCodes/fiducial">Fiducial</a>
        </p>
      </div>
    </footer>
  );
}
