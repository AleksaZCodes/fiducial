import { type Locale, defaultLocale, locales } from "@/generated/messages";

export function isLocale(value: string): value is Locale {
  return (locales as readonly string[]).includes(value);
}

/**
 * The URL of a page in a locale: `/details` in the default locale, `/sr/details`
 * in another. `src/routing.ts` holds the rewrite that makes the bare form work.
 */
export function localePath(locale: Locale, path: string): string {
  const prefix = locale === defaultLocale ? "" : `/${locale}`;
  if (path === "/") return prefix || "/";
  return `${prefix}${path}`;
}
