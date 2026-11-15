import type { Locale } from "@/generated/messages";
import { localePath } from "@/lib/routes";
import { t } from "@/lib/t";
import Link from "next/link";
import { SiteLocalePicker } from "./site-locale-picker";

export function SiteHeader({ locale }: { locale: Locale }) {
  return (
    <header className="sticky top-0 z-10 border-b border-border bg-background/90 backdrop-blur">
      <div className="mx-auto flex w-full max-w-3xl items-center justify-between gap-4 px-6 py-3">
        <Link href={localePath(locale, "/")} className="type-h3">
          {t(locale, "meta.title")}
        </Link>
        <SiteLocalePicker locale={locale} />
      </div>
    </header>
  );
}
