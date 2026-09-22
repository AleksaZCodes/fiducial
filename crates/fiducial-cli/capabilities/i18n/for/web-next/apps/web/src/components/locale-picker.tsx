"use client";

import { Menu } from "@base-ui-components/react/menu";
import { type Locale, locales, messages } from "@/generated/messages";
import { queryLocaleHref } from "@/lib/locale-href";
import * as Flags from "country-flag-icons/react/3x2";
import Link from "next/link";
import { usePathname, useSearchParams } from "next/navigation";
import styles from "./locale-picker.module.css";

/**
 * The language picker.
 *
 * Its Svelte twin is
 * `i18n/for/web-svelte/apps/web/src/lib/components/LocalePicker.svelte` and
 * the two are the same decisions in two frameworks. Change one, change both.
 *
 * ## Why this is a list and not a toggle
 *
 * It used to be a single link to "the other locale", reading its label from a
 * `nav.otherLang` key. That works for exactly two locales and silently becomes
 * wrong at three: with `sr`, `en` and `de` installed, "the other one" is not a
 * thing, and the key holding one language's name inside another language's
 * catalog has nowhere to put the third. The key is gone and so is the toggle.
 *
 * ## Every locale names itself
 *
 * A language is listed under its **endonym** — the name it uses for itself.
 * `Srpski`, not `Serbian`; `Deutsch`, not `German`. This is not a stylistic
 * preference. A picker exists to be used by someone who cannot read the page
 * they are currently looking at, and translating the list into the language
 * they are trying to leave is the one arrangement guaranteed to be useless to
 * them.
 *
 * So each catalog declares its own name and its own flag region:
 *
 * ```json
 * // messages/sr.json
 * "locale": { "name": "Srpski", "region": "RS", "tag": "sr-Latn-RS" }
 * ```
 *
 * and this component reads `messages[l]["locale.name"]` for each `l` in
 * `locales`. There is no table of languages here, nothing to extend, and no
 * branch anywhere on a locale code. Adding `messages/de.json` with a `locale`
 * block puts German in this menu and nothing else has to be touched — which is
 * the whole test of whether an i18n layer is real.
 *
 * ## The flag is decoration, and is marked as such
 *
 * A flag is a country, not a language, and the two do not line up: Serbian is
 * not confined to Serbia and English is not confined to anywhere. The flag is
 * `aria-hidden` and the endonym is the label, so assistive tech announces the
 * language and never the nation. `region` is a hint for the icon set, not a
 * claim about who speaks what.
 *
 * A locale whose declared region has no flag in the set renders as no flag
 * rather than as a broken image, and the row still works — the name was always
 * the thing carrying the meaning.
 *
 * ## Why Base UI, and not the copied shadcn menu
 *
 * This template used to import `@/components/ui/dropdown-menu` and
 * `@/components/ui/button`. Those are files the `design` capability *copies
 * into* a product, so the import resolved only where that copy had happened —
 * and the failure was a build error inside a file the product never wrote,
 * blaming the capability that did nothing wrong. A capability template may
 * depend on a package; it may not depend on another capability's copied file.
 * The Svelte twin has the same constraint and answers it the same way.
 */
export type LocalePickerVariant =
  /** Flag, endonym, chevron. The default, and the one to use in a footer. */
  | "full"
  /** Flag only. For a dense bar where the name costs more than it earns. */
  | "icon";

export function LocalePicker({
  locale,
  variant = "full",
  href,
}: {
  locale: Locale;
  variant?: LocalePickerVariant;
  /**
   * Where a language leads. Defaults to `?lang=`, which needs no routing at
   * all. A product with locale-prefixed routes passes a builder over
   * `prefixLocaleHref` — see `@/lib/locale-href` for why this is a strategy
   * rather than a rule.
   */
  href?: (target: Locale) => string;
}) {
  const pathname = usePathname();
  const search = useSearchParams().toString();
  const url = `${pathname}${search ? `?${search}` : ""}`;
  const hrefFor = href ?? ((target: Locale) => queryLocaleHref(url, target));
  const icon = variant === "icon";

  return (
    <Menu.Root>
      {/* `aria-label` carries the name in BOTH variants, so the icon-only
          trigger is not a mystery button to a screen reader. That is the whole
          cost of dropping the visible name, and it is why the name is never
          the only thing announcing what this control is. */}
      <Menu.Trigger
        className={styles.trigger}
        data-variant={variant}
        aria-label={messages[locale]["locale.switch"]}
        title={icon ? messages[locale]["locale.name"] : undefined}
      >
        <FlagFrame l={locale} />
        {icon ? null : messages[locale]["locale.name"]}
        {icon ? null : (
          <svg viewBox="0 0 16 16" aria-hidden="true" className={styles.chevron}>
            <path d="M4 6 L8 10 L12 6" fill="none" stroke="currentColor" strokeWidth="1.5" />
          </svg>
        )}
      </Menu.Trigger>

      <Menu.Portal>
        <Menu.Positioner align="end" sideOffset={6}>
          <Menu.Popup className={styles.menu}>
            {locales.map((l) => {
              const current = l === locale;
              return (
                <Menu.Item
                  key={l}
                  // `render` is Base UI's delegation prop — Radix's `asChild`.
                  // The row stays a real link, so it is middle-clickable,
                  // copyable and crawlable, and `hrefLang` states the
                  // relationship outright. A locale switcher built from click
                  // handlers is invisible to a search engine and to anyone
                  // opening it in a new tab.
                  render={
                    <Link
                      href={hrefFor(l)}
                      hrefLang={l}
                      lang={l}
                      aria-current={current ? "true" : undefined}
                      className={styles.item}
                    />
                  }
                >
                  <FlagFrame l={l} />
                  <span className={styles.name}>{messages[l]["locale.name"]}</span>
                  {current ? (
                    <svg viewBox="0 0 16 16" aria-hidden="true" className={styles.check}>
                      <path
                        d="M3 8.5 L6.5 12 L13 4"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="1.75"
                      />
                    </svg>
                  ) : null}
                </Menu.Item>
              );
            })}
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  );
}

/**
 * How a flag is drawn, everywhere: a 3:2 frame with a hairline border.
 *
 * The frame is a `<span>`, not the `<svg>`, and that is the fix for a bug that
 * drew the border as a SQUARE. Menu items and buttons in most UI kits size
 * every child svg through a descendant selector that outranks a plain
 * `h-*`/`w-*` utility on the svg itself. So the flag's own 3:2 dimensions were
 * overridden, the flag letterboxed inside a square, and the border traced the
 * square. The span owns the box; the svg just fills it.
 */
function FlagFrame({ l }: { l: Locale }) {
  const Flag = flagFor(l);
  if (!Flag) return null;
  return (
    <span aria-hidden="true" className={styles.flag}>
      <Flag />
    </span>
  );
}

/**
 * The flag component for a locale's declared region, or undefined.
 *
 * The lookup is on the catalog's own `locale.region`, so this function knows
 * nothing about any particular language and never needs editing.
 */
function flagFor(l: Locale) {
  const region = messages[l]["locale.region"];
  return (Flags as Record<string, React.ComponentType | undefined>)[region];
}
