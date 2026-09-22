import { type Locale } from "@/generated/messages";
import { formatContentDate } from "@/lib/intl";
import Link from "next/link";
import styles from "./entry-cards.module.css";

/**
 * A collection's entries: one cover story, then a grid of the rest.
 *
 * Its Svelte twin is
 * `content/for/web-svelte/apps/web/src/lib/components/EntryCards.svelte`.
 * Change one, change both.
 *
 * ## Why a cover story
 *
 * A list of identical cards says every entry is equally worth reading, which
 * is never true and is least true of a blog's front page. The most recent (or
 * pinned) entry gets a card with room for its summary; the rest get a grid
 * below. That is the whole of the hierarchy, and it is one decision rather
 * than a design system.
 *
 * With a single entry there is no grid — a section divider with nothing under
 * it reads as a loading failure.
 *
 * ## The whole card is one link
 *
 * Not the title with the card inert around it. A card that only responds on
 * one line is a card people click on the wrong part of, and they do not
 * conclude the site has a small hit target — they conclude it is broken.
 * `<a>` wrapping block content is valid HTML5 as long as nothing inside is
 * itself interactive, which is why the "read more" line is a `<span>` and not
 * a second link.
 *
 * ## What is the product's
 *
 * `entries` and `href` come in. Which collection this is, where it lives, and
 * what the section is called are product facts; the shape of a card is not.
 * No `"use client"`: there is no state here, so this stays a server component
 * and ships no JavaScript.
 */
export type Entry = {
  slug: string;
  title: string;
  /** `YYYY-MM-DD`, as every entry's frontmatter stores it. */
  date?: string;
  summary?: string;
  tags?: readonly string[];
};

export function EntryCards({
  entries,
  locale,
  href,
  readMoreLabel,
}: {
  entries: readonly Entry[];
  locale: Locale;
  href: (entry: Entry) => string;
  /** e.g. `t("blog.readMore")`. Omit to leave the line off. */
  readMoreLabel?: string;
}) {
  const [cover, ...rest] = entries;

  return (
    <>
      {cover ? (
        <Link className={styles.cover} href={href(cover)}>
          {cover.date ? (
            <time className={styles.meta} dateTime={cover.date}>
              {formatContentDate(cover.date, locale)}
            </time>
          ) : null}
          <h2 className={styles.coverTitle}>{cover.title}</h2>
          {cover.summary ? <p className={styles.summary}>{cover.summary}</p> : null}
          {readMoreLabel ? <span className={styles.more}>{readMoreLabel} →</span> : null}
        </Link>
      ) : null}

      {rest.length > 0 ? (
        <ul className={styles.grid}>
          {rest.map((entry) => (
            <li key={entry.slug}>
              <Link className={styles.card} href={href(entry)}>
                {entry.date ? (
                  <time className={styles.meta} dateTime={entry.date}>
                    {formatContentDate(entry.date, locale)}
                  </time>
                ) : null}
                <h3 className={styles.cardTitle}>{entry.title}</h3>
                {entry.summary ? <p className={styles.summary}>{entry.summary}</p> : null}
              </Link>
            </li>
          ))}
        </ul>
      ) : null}
    </>
  );
}
