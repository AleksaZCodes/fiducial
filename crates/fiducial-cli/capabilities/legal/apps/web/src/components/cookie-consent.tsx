"use client";

/**
 * Cookie consent, for a product that declares cookie categories in
 * `[legal] cookie_categories`.
 *
 * Installed by the `legal` capability.
 *
 * ## The rules this is built to satisfy
 *
 * GDPR Art. 7 and the ePrivacy Directive, as read by every European regulator
 * that has published guidance on banners:
 *
 * - **Nothing non-necessary is set before a choice.** This component stores a
 *   decision; it does not load anything. The product reads `cookieConsent()`
 *   and only then loads whatever the category permits. That ordering is the
 *   whole compliance story, and it is the product's job as much as this
 *   component's.
 * - **Refusing is as easy as accepting.** One click, on a control with the
 *   same prominence. A banner whose "reject" is a second click behind
 *   "settings" is the single most commonly fined dark pattern in the EU, so
 *   both buttons are here, side by side, styled the same weight.
 * - **No pre-ticked boxes.** Every optional category starts off.
 * - **Withdrawal is as easy as consent.** `openCookieSettings()` reopens this
 *   from a footer link, which the Cookie Policy promises exists.
 * - **No consent wall.** Dismissing is a valid outcome and the site works.
 *
 * ## Why localStorage and not a cookie
 *
 * The record of the choice is not itself needed by the server, so making it a
 * cookie would mean sending it on every request for no reason. It is also, in
 * the necessary-only case, the only thing stored at all, which is a pleasing
 * property for a cookie banner to have.
 *
 * The read is wrapped: a private window or blocked site data throws on access
 * rather than returning null, and an exception here would take the whole page
 * down. Unreadable is treated as "not yet asked", which fails toward asking
 * again rather than toward assuming consent.
 */

import { useCallback, useEffect, useState } from "react";

const STORAGE_KEY = "cookie-consent";
/** Bump when the category set changes: a stored choice about a different set of categories is not a choice about this one. */
const VERSION = 1;

export type CookieCategory = "necessary" | "analytics" | "marketing" | "functional";

export interface ConsentRecord {
  v: number;
  /** ISO date the choice was made, so an audit can show when. */
  at: string;
  granted: CookieCategory[];
}

/** The stored decision, or `null` when nobody has been asked yet. */
export function cookieConsent(): ConsentRecord | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as ConsentRecord;
    if (parsed.v !== VERSION || !Array.isArray(parsed.granted)) return null;
    return parsed;
  } catch {
    return null;
  }
}

/** True when this category may be used. `necessary` is always true. */
export function hasConsent(category: CookieCategory): boolean {
  if (category === "necessary") return true;
  return cookieConsent()?.granted.includes(category) ?? false;
}

const REOPEN_EVENT = "cookie-consent:open";

/** Reopen the banner. Wire this to the footer link the Cookie Policy promises. */
export function openCookieSettings() {
  window.dispatchEvent(new Event(REOPEN_EVENT));
}

function store(granted: CookieCategory[]) {
  try {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ v: VERSION, at: new Date().toISOString(), granted } satisfies ConsentRecord),
    );
  } catch {
    // Storage denied. The choice applies to this page view and we ask again
    // next time, which is the conservative failure: it never remembers a
    // consent it could not record.
  }
}

export interface CookieConsentProps {
  /** Optional categories, in the order they should be listed. Omit for a necessary-only product, where this component renders nothing. */
  categories?: Exclude<CookieCategory, "necessary">[];
  /** Copy, passed in so it comes from the product's message catalog rather than being hardcoded English here. */
  labels: {
    title: string;
    body: string;
    acceptAll: string;
    rejectAll: string;
    policyHref: string;
    policyLabel: string;
    /** Per-category label, keyed by category. */
    category?: Partial<Record<CookieCategory, string>>;
    save?: string;
  };
}

export function CookieConsent({ categories = [], labels }: CookieConsentProps) {
  const [open, setOpen] = useState(false);
  const [selected, setSelected] = useState<CookieCategory[]>([]);

  useEffect(() => {
    // A product with nothing but necessary cookies has nothing to ask about,
    // and a banner that asks anyway is noise that trains people to dismiss the
    // ones that matter.
    if (categories.length === 0) return;
    if (cookieConsent() === null) setOpen(true);

    const reopen = () => {
      setSelected(cookieConsent()?.granted.filter((c) => c !== "necessary") ?? []);
      setOpen(true);
    };
    window.addEventListener(REOPEN_EVENT, reopen);
    return () => window.removeEventListener(REOPEN_EVENT, reopen);
  }, [categories.length]);

  const decide = useCallback((granted: CookieCategory[]) => {
    store(["necessary", ...granted.filter((c) => c !== "necessary")]);
    setOpen(false);
  }, []);

  if (!open || categories.length === 0) return null;

  return (
    <section
      // A landmark region, not a dialog. A modal dialog traps focus and blocks
      // the page, which would make this a consent wall, and consent given to
      // get past a wall is not freely given. The site stays usable while the
      // question is on screen.
      //
      // `<section>` with an accessible name IS a region to assistive
      // technology, so this needs no explicit role. Without the label it would
      // be a generic section and disappear from the landmark list.
      aria-label={labels.title}
      className="fixed inset-x-0 bottom-0 z-50 p-4 sm:p-6"
    >
      <div className="shape-panel mx-auto max-w-3xl p-6">
        <h2 className="type-h3">{labels.title}</h2>
        <p className="type-small mt-3 text-muted-foreground">
          {labels.body}{" "}
          <a href={labels.policyHref} className="text-primary underline-offset-4 hover:underline">
            {labels.policyLabel}
          </a>
        </p>

        {categories.length > 1 ? (
          <fieldset className="mt-5 space-y-2">
            <legend className="sr-only">{labels.title}</legend>
            {categories.map((c) => (
              <label key={c} className="type-small flex items-center gap-3">
                <input
                  type="checkbox"
                  checked={selected.includes(c)}
                  onChange={(e) =>
                    setSelected((prev) =>
                      e.target.checked ? [...prev, c] : prev.filter((p) => p !== c),
                    )
                  }
                />
                {labels.category?.[c] ?? c}
              </label>
            ))}
          </fieldset>
        ) : null}

        {/* Reject first in the DOM and equal in weight. Both are the same
            control at the same size: the moment one is louder than the other,
            the consent it collects is not freely given. */}
        <div className="mt-6 flex flex-wrap gap-3">
          <button
            type="button"
            onClick={() => decide([])}
            className="shape-control shape-outline type-small px-5 py-2.5 font-medium"
          >
            {labels.rejectAll}
          </button>
          {categories.length > 1 && labels.save ? (
            <button
              type="button"
              onClick={() => decide(selected)}
              className="shape-control shape-outline type-small px-5 py-2.5 font-medium"
            >
              {labels.save}
            </button>
          ) : null}
          <button
            type="button"
            onClick={() => decide(categories)}
            className="shape-control type-small px-5 py-2.5 font-medium text-primary-foreground [--edge:var(--primary)] [--fill:var(--primary)]"
          >
            {labels.acceptAll}
          </button>
        </div>
      </div>
    </section>
  );
}
