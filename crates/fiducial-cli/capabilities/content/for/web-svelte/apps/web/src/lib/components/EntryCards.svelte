<script lang="ts">
  /**
   * A collection's entries: one cover story, then a grid of the rest.
   *
   * ## Why a cover story
   *
   * A list of identical cards says every entry is equally worth reading,
   * which is never true and is least true of a blog's front page. The most
   * recent (or pinned) entry gets a card with room for its summary; the rest
   * get a grid below. That is the whole of the hierarchy, and it is one
   * decision rather than a design system.
   *
   * With a single entry there is no grid and no rule under the cover — a
   * section divider with nothing under it reads as a loading failure.
   *
   * ## The whole card is one link
   *
   * Not the title with the card inert around it. A card that only responds on
   * one line is a card people click on the wrong part of, and they do not
   * conclude the site has a small hit target — they conclude it is broken.
   * `<a>` wrapping block content is valid HTML5 as long as nothing inside is
   * itself interactive, which is why the "read more" line is a `<span>` and
   * not a second link.
   *
   * ## What is the product's
   *
   * `entries` and `href` come in. Which collection this is, where it lives,
   * and what the section is called are product facts; the shape of a card is
   * not. The date is formatted through the shared Intl binding, so it reads
   * `24. septembar 2026.` or `24 September 2026` without this component
   * knowing either language.
   */
  import { formatContentDate } from "../intl";
  import type { Locale } from "../../generated/messages";

  type Entry = {
    slug: string;
    title: string;
    /** `YYYY-MM-DD`, as every entry's frontmatter stores it. */
    date?: string;
    summary?: string;
    tags?: readonly string[];
  };

  let {
    entries,
    locale,
    href,
    readMoreLabel,
  }: {
    entries: readonly Entry[];
    locale: Locale;
    href: (entry: Entry) => string;
    /** e.g. `t(locale, "blog.readMore")`. Omit to leave the line off. */
    readMoreLabel?: string;
  } = $props();

  const cover = $derived(entries[0]);
  const rest = $derived(entries.slice(1));
</script>

{#if cover}
  <a class="fid-cover" href={href(cover)}>
    {#if cover.date}
      <time class="fid-meta" datetime={cover.date}>{formatContentDate(cover.date, locale)}</time>
    {/if}
    <h2 class="fid-cover-title">{cover.title}</h2>
    {#if cover.summary}
      <p class="fid-summary">{cover.summary}</p>
    {/if}
    {#if readMoreLabel}
      <span class="fid-more">{readMoreLabel} →</span>
    {/if}
  </a>
{/if}

{#if rest.length > 0}
  <ul class="fid-grid">
    {#each rest as entry (entry.slug)}
      <li>
        <a class="fid-card" href={href(entry)}>
          {#if entry.date}
            <time class="fid-meta" datetime={entry.date}>
              {formatContentDate(entry.date, locale)}
            </time>
          {/if}
          <h3 class="fid-card-title">{entry.title}</h3>
          {#if entry.summary}
            <p class="fid-summary">{entry.summary}</p>
          {/if}
        </a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  /* Scoped, unlike the picker and the sheet: nothing here is portalled, so
     Svelte's own scoping reaches all of it. */
  .fid-cover,
  .fid-card {
    display: block;
    padding: 1.5rem;
    border: 1px solid var(--border, currentColor);
    border-radius: var(--radius-md, 4px);
    color: inherit;
    text-decoration: none;
    height: 100%;
    transition: border-color 120ms ease;
  }
  .fid-cover:hover,
  .fid-card:hover {
    border-color: var(--primary, currentColor);
  }
  .fid-cover-title {
    margin: 0.25rem 0 0;
    font-size: 1.75rem;
    line-height: 1.15;
  }
  .fid-card-title {
    margin: 0.25rem 0 0;
    font-size: 1.125rem;
    line-height: 1.25;
  }
  .fid-meta {
    font-size: 0.8125rem;
    color: var(--muted-foreground, inherit);
    opacity: 0.8;
  }
  .fid-summary {
    margin: 0.625rem 0 0;
    max-width: 60ch;
    color: var(--muted-foreground, inherit);
  }
  .fid-more {
    display: inline-block;
    margin-top: 0.75rem;
    color: var(--primary, inherit);
  }
  .fid-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
    gap: 1rem;
    margin: 1rem 0 0;
    padding: 0;
    list-style: none;
  }
</style>
