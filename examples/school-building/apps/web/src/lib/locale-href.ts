/**
 * Where "the same page, in another language" lives.
 *
 * ## Why this is a strategy and not a rule
 *
 * There are two working answers and the platform does not get to pick one:
 *
 * - **query** — `/press?lang=en`. No routing to set up, one cookie or one
 *   search param, works on a product that has no locale routes at all.
 * - **prefix** — `/en/press`, with the default locale owning the bare path.
 *   Crawlable, linkable, cacheable per language; needs the router to carry a
 *   locale segment.
 *
 * Which one a product uses is a routing decision, made once, and a component
 * has no business hard-coding either. So the picker takes an `href` builder,
 * and these are the two builders — the default being `query`, because it is
 * the one that works before any routing exists.
 *
 * ## The rule that is not negotiable
 *
 * **Switching language must keep the reader on the page they are on.** A
 * picker that links to the locale's home page silently teleports somebody who
 * has just told you they cannot read the current page — the one person least
 * able to find their way back. Both builders below preserve the path; a third
 * one must too.
 */

import { type Locale, defaultLocale, locales } from "../generated/messages";

/** `?lang=<locale>` on the current path, preserving every other parameter. */
export function queryLocaleHref(url: URL | Location | string, target: Locale): string {
  const u = new URL(typeof url === "string" ? url : url.href, "https://placeholder.invalid");
  u.searchParams.set("lang", target);
  return `${u.pathname}?${u.searchParams.toString()}${u.hash}`;
}

/**
 * `/<locale>/<path>`, with the default locale owning the bare path.
 *
 * Derived from the declared locale set rather than from a literal `/en`, so
 * adding a language to `[i18n] locales` needs no edit here.
 */
export function prefixLocaleHref(
  url: URL | Location | string,
  target: Locale,
  current: Locale,
): string {
  const u = new URL(typeof url === "string" ? url : url.href, "https://placeholder.invalid");
  const prefixOf = (l: Locale) => (l === defaultLocale ? "" : `/${l}`);

  let rest = u.pathname;
  const from = prefixOf(current);
  if (from && (rest === from || rest.startsWith(`${from}/`))) {
    rest = rest.slice(from.length);
  }
  // A path that carries *some* locale prefix, but not the one we were told is
  // current, still has to lose it — otherwise a stale `current` produces
  // `/en/sr/press`, which 404s.
  for (const l of locales) {
    const p = prefixOf(l);
    if (p && (rest === p || rest.startsWith(`${p}/`))) {
      rest = rest.slice(p.length);
      break;
    }
  }
  if (!rest.startsWith("/")) rest = `/${rest}`;

  const path = `${prefixOf(target)}${rest === "/" ? "/" : rest}`.replace(/\/$/, "") || "/";
  return `${path}${u.search}${u.hash}`;
}
