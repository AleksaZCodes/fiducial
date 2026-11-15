"use client";

import { LocalePicker } from "@/components/locale-picker";
import type { Locale } from "@/generated/messages";
import { prefixLocaleHref } from "@/lib/locale-href";
import { usePathname } from "next/navigation";

/** The language picker, keeping the reader on the page they are on. */
export function SiteLocalePicker({ locale }: { locale: Locale }) {
  const pathname = usePathname();
  return (
    <LocalePicker locale={locale} href={(target) => prefixLocaleHref(pathname, target, locale)} />
  );
}
