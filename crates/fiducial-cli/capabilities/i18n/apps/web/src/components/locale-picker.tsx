"use client";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { type Locale, locales, messages } from "@/generated/messages";
import { switchLocalePath } from "@/lib/i18n";
import * as Flags from "country-flag-icons/react/3x2";
import { CheckIcon, ChevronDownIcon } from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";

/**
 * The language picker.
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
 * "locale": { "name": "Srpski", "region": "RS" }
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
 */
export type LocalePickerVariant =
  /** Flag, endonym, chevron. The default, and the one to use in a footer. */
  | "full"
  /** Flag only. For a dense bar where the name costs more than it earns. */
  | "icon";

export function LocalePicker({
  locale,
  variant = "full",
}: {
  locale: Locale;
  variant?: LocalePickerVariant;
}) {
  const pathname = usePathname();
  const label = messages[locale]["nav.langLabel"];
  const icon = variant === "icon";

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        {/* `aria-label` carries the name in BOTH variants, so the icon-only
            trigger is not a mystery button to a screen reader. That is the
            whole cost of dropping the visible name, and it is why the name is
            never the only thing announcing what this control is. */}
        <Button
          variant="outline"
          size={icon ? "icon-sm" : "sm"}
          aria-label={label}
          title={icon ? messages[locale]["locale.name"] : undefined}
          className={
            icon
              ? "text-muted-foreground hover:text-foreground"
              : "gap-2 text-muted-foreground hover:text-foreground"
          }
        >
          <FlagFrame l={locale} />
          {icon ? null : messages[locale]["locale.name"]}
          {icon ? null : <ChevronDownIcon aria-hidden="true" className="opacity-60" />}
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="end" className="min-w-44">
        <DropdownMenuGroup>
          {locales.map((l) => {
            const current = l === locale;
            return (
              <DropdownMenuItem key={l} asChild>
                {/* A real link, so the row is middle-clickable, copyable and
                    crawlable — a locale switcher that is a click handler is
                    invisible to a search engine and to anyone opening it in a
                    new tab. `hrefLang` states the relationship outright. */}
                <Link
                  href={switchLocalePath(pathname, locale, l)}
                  hrefLang={l}
                  lang={l}
                  aria-current={current}
                >
                  <FlagFrame l={l} />
                  <span className="flex-1">{messages[l]["locale.name"]}</span>
                  {current ? <CheckIcon aria-hidden="true" /> : null}
                </Link>
              </DropdownMenuItem>
            );
          })}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

/**
 * The flag component for a locale's declared region, or null.
 *
 * The lookup is on the catalog's own `locale.region`, so this function knows
 * nothing about any particular language and never needs editing.
 */
/**
 * How a flag is drawn, everywhere: a 3:2 frame with a hairline border.
 *
 * The frame is a `<span>`, not the `<svg>`, and that is the fix for a bug that
 * drew the border as a SQUARE. Button and DropdownMenuItem both size every
 * child svg that lacks a `size-*` class to `size-4` — 16 by 16 — through a
 * descendant selector that outranks a plain `h-*`/`w-*` utility on the svg. So
 * the flag's own 3:2 dimensions were overridden, the flag letterboxed inside a
 * square, and the border traced the square.
 *
 * The span owns the 3:2 box and the border; the svg fills it with `size-full`,
 * whose class name contains `size-` and so opts out of the parents' rule.
 */
function FlagFrame({ l }: { l: Locale }) {
  const Flag = flagFor(l);
  if (!Flag) return null;
  return (
    <span
      aria-hidden="true"
      className="inline-flex h-3.5 w-[1.3125rem] shrink-0 overflow-hidden rounded-[1px] border border-border"
    >
      <Flag className="size-full" />
    </span>
  );
}

function flagFor(l: Locale) {
  const region = messages[l]["locale.region"];
  return (Flags as Record<string, React.ComponentType<{ className?: string }> | undefined>)[region];
}
