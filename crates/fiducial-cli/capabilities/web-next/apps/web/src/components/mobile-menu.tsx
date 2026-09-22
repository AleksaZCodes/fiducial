"use client";

import Link from "next/link";
// A CSS module, not a global stylesheet: Next.js only allows global CSS from
// the root layout, so a template that imported one would fail to build in the
// product that installed it and nowhere else.
import styles from "./mobile-menu.module.css";
import { useEffect, useId, useRef, useState } from "react";

/**
 * The nav links on a narrow screen, in a sheet that slides in from the right.
 *
 * ## Why this is not a shadcn `Sheet`
 *
 * The obvious implementation imports `@/components/ui/sheet`, which is what
 * the app this was generalized from does. A capability template cannot: a
 * shadcn component is *copied into* a product, so the import resolves only in
 * products that ran that copy, and the failure is a build error in a file the
 * product never wrote. The Svelte twin has the same constraint and solves it
 * the same way — one primitive, no UI-kit dependency.
 *
 * So this is the dialog behaviour spelled out, and the list is short because
 * the list is the point:
 *
 * - `<dialog>` with `showModal()`, so the top layer, the backdrop, the inert
 *   page behind it and Escape are all the browser's, not ours.
 * - Focus returns to the trigger on close — `<dialog>` does that; the
 *   hand-rolled version this replaces did not.
 * - Every link closes the sheet. This matters most for a fragment link like
 *   `/#contact`, which navigates without a page load, so nothing else would
 *   ever dismiss it and the reader is left staring at a menu over the section
 *   they asked for.
 *
 * ## What is the product's, and what is not
 *
 * This owns the sheet. It owns no words and no routes: `links` and both
 * labels come in, because which pages a product has is the product's fact and
 * translating them is the catalog's job. A menu that hard-codes four `href`s
 * is a menu every product forks; one that takes twenty props to avoid it is
 * worse. Composition stays out too — there is deliberately no `SiteNav` in
 * the platform, because arranging a wordmark, links, a language picker and a
 * call to action is the decision each product makes differently.
 */
export function MobileMenu({
  links,
  openLabel,
  closeLabel,
  className = "",
}: {
  links: readonly { href: string; label: string }[];
  openLabel: string;
  closeLabel: string;
  /** Extra classes on the trigger — e.g. `md:hidden`. */
  className?: string;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [open, setOpen] = useState(false);
  const titleId = useId();

  // `open` is React's state; `showModal()`/`close()` is the element's. Driving
  // the element from an effect keeps one of them authoritative — setting the
  // `open` attribute directly renders a non-modal dialog with no backdrop and
  // no focus trap, which looks right and is not.
  useEffect(() => {
    const el = dialog.current;
    if (!el) return;
    if (open && !el.open) el.showModal();
    if (!open && el.open) el.close();
  }, [open]);

  return (
    <>
      <button
        type="button"
        aria-label={openLabel}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => setOpen(true)}
        className={`${styles.burger} ${className}`}
      >
        {/* Drawn rather than pulled from an icon set: two paths beat a
            dependency, and square ends match an unrounded design system. */}
        <svg viewBox="0 0 16 16" aria-hidden="true" className={styles.burgerIcon}>
          <path d="M2 4 H14 M2 8 H14 M2 12 H14" stroke="currentColor" strokeWidth="1.75" />
        </svg>
      </button>

      <dialog
        ref={dialog}
        className={styles.sheet}
        aria-labelledby={titleId}
        // Fires for Escape and for a backdrop dismissal, so React's state
        // cannot drift out of step with the element's.
        onClose={() => setOpen(false)}
        // The backdrop is part of the dialog's own box, so a click on it
        // targets the dialog itself — that is how you tell the two apart.
        onClick={(e) => {
          if (e.target === dialog.current) setOpen(false);
        }}
      >
        <h2 id={titleId} className={styles.srOnly}>
          {openLabel}
        </h2>
        <button
          type="button"
          aria-label={closeLabel}
          onClick={() => setOpen(false)}
          className={styles.sheetClose}
        >
          <svg viewBox="0 0 16 16" aria-hidden="true" className={styles.burgerIcon}>
            <path d="M3 3 L13 13 M13 3 L3 13" stroke="currentColor" strokeWidth="1.75" />
          </svg>
        </button>
        <ul className={styles.sheetList}>
          {links.map((l) => (
            <li key={l.href}>
              <Link href={l.href} className={styles.sheetLink} onClick={() => setOpen(false)}>
                {l.label}
              </Link>
            </li>
          ))}
        </ul>
      </dialog>
    </>
  );
}
