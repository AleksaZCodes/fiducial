<script lang="ts">
  /**
   * The press room.
   *
   * Its React twin is `press/for/web-next/.../press-kit.tsx`, and the two are
   * the same decisions in two frameworks. Change one, change both.
   *
   * ## The order is the order a journalist needs things in
   *
   *   1. **Assets** — the logo and the gallery. Usually the reason they came:
   *      they already have a story and need a picture for it.
   *   2. **Fast facts** — the table they check their copy against.
   *   3. **Boilerplate** — the paragraph they paste at the bottom.
   *   4. **Stories** — for the ones who have not got an angle yet.
   *   5. **Contact** — last, because by now they know what to ask.
   *
   * It was ordered the other way round in both of the apps this was
   * generalized from, prose first, which puts the one thing most visitors
   * want below three screens of text.
   *
   * ## What the product supplies
   *
   * Labels (from its catalog), the logo list, the gallery, and three
   * functions: where a story lives, how an angle reads, and how a date reads.
   * The room's *content* comes from `generated/press.ts`, which is plain data
   * — so this component never reaches for a declaration itself.
   */
  import MediaFigure from "./MediaFigure.svelte";
  import { mediaUrl } from "../media";
  import { press, pressFor, type PressLocale } from "../../generated/press";

  type LabelKey =
    | "title"
    | "assets"
    | "guidelines"
    | "facts"
    | "boilerplate"
    | "short"
    | "medium"
    | "long"
    | "stories"
    | "contact"
    | "download";

  let {
    locale,
    labels,
    logos,
    gallery = [],
    storyHref,
    angleLabel = (a: string) => a,
    formatDate,
  }: {
    locale: PressLocale;
    labels: Record<LabelKey, string>;
    /**
     * The brand primitives, previewed and downloadable. Served from the repo
     * rather than the media bucket — they are source files, not content.
     *
     * `background` is not decoration. A mark drawn in light colours on the
     * ambient page background is invisible, and that shipped: every mark was
     * put on the one surface it could not be read against. A file name
     * usually names the surface a mark is drawn *for*, not the mark's own
     * colour, so check the fills rather than the name.
     */
    logos: readonly { label: string; href: string; background?: string }[];
    /** Pictures from the media bucket, with their localized captions. */
    gallery?: readonly { key: string; alt: string; caption: string }[];
    storyHref: (slug: string) => string;
    angleLabel?: (angle: string) => string;
    formatDate: (iso: string) => string;
  } = $props();

  const room = $derived(pressFor(locale));
  const boilerplateKeys = ["short", "medium", "long"] as const;
  const rowsFor = (k: string) => (k === "long" ? 9 : k === "medium" ? 5 : 3);
</script>

<div class="kit">
  <h1>{labels.title}</h1>

  <section>
    <h2>{labels.assets}</h2>
    <p class="muted">{labels.guidelines}</p>
    <ul class="grid">
      {#each logos as logo (logo.href)}
        <li>
          <div class="logo-frame" style:background={logo.background}>
            <img src={logo.href} alt={logo.label} />
          </div>
          <div class="row">
            <span class="cap">{logo.label}</span>
            <!-- `download` is enough here: these are same-origin repository
                 files, not media-route URLs that need `?download`. -->
            <a class="dl" href={logo.href} download>{labels.download}</a>
          </div>
        </li>
      {/each}
      {#each gallery as g (g.key)}
        <li>
          <MediaFigure
            mediaKey={g.key}
            alt={g.alt}
            caption={g.caption}
            downloadLabel={labels.download}
            fit="contain"
          />
        </li>
      {/each}
    </ul>
  </section>

  <section>
    <h2>{labels.facts}</h2>
    <dl class="facts">
      {#each room.facts as fact (fact.label)}
        <div class="fact">
          <dt>{fact.label}</dt>
          <dd>{fact.value}</dd>
        </div>
      {/each}
    </dl>
  </section>

  <section>
    <h2>{labels.boilerplate}</h2>
    {#each boilerplateKeys as k (k)}
      {#if room.boilerplate[k]}
        <div class="boiler">
          <h3>{labels[k]}</h3>
          <!-- A read-only field rather than a paragraph: it selects cleanly on
               click and copies as plain text, which is the whole job. A <p>
               copies with the page's markup attached. -->
          <textarea readonly aria-label={labels[k]} rows={rowsFor(k)} value={room.boilerplate[k]}
          ></textarea>
        </div>
      {/if}
    {/each}
  </section>

  {#if room.stories.length > 0}
    <section>
      <h2>{labels.stories}</h2>
      <ul class="grid">
        {#each room.stories as s (s.slug)}
          <li>
            <!-- One link for the whole card, not just the title: a card that
                 looks clickable and only responds on one line is a card people
                 click on the wrong part of. -->
            <a class="story" href={storyHref(s.slug)}>
              {#if s.cover}
                <div class="cover">
                  <img src={mediaUrl(s.cover)} alt={s.coverAlt} loading="lazy" />
                </div>
              {/if}
              <p class="meta">
                {#if s.angle}<span>{angleLabel(s.angle)}</span>{/if}
                {#if s.date}<time datetime={s.date}>{formatDate(s.date)}</time>{/if}
              </p>
              <h3>{s.title}</h3>
              {#if s.summary}<p class="muted">{s.summary}</p>{/if}
            </a>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  <section>
    <h2>{labels.contact}</h2>
    <p><a href={`mailto:${press.pressEmail}`}>{press.pressEmail}</a></p>
    {#if press.spokesperson}<p class="muted">{press.spokesperson}</p>{/if}
  </section>
</div>

<style>
  .kit {
    display: flex;
    flex-direction: column;
    gap: 3.5rem;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  /* Explicit sizes, in `em` so they scale with the product's own type.
     These are bare `h1`/`h2`/`h3`, and an app stylesheet that resets heading
     sizes — which is a normal thing to do when a design system supplies
     `.type-h2` classes instead — flattens this whole page to body text. The
     hierarchy is the component's job, the typeface is the product's. */
  h1 {
    margin: 0;
    font-size: 2em;
    line-height: 1.1;
  }
  h2 {
    margin: 0;
    font-size: 1.4em;
    line-height: 1.2;
  }
  h3 {
    margin: 0;
    font-size: 1.1em;
    line-height: 1.25;
  }
  p {
    margin: 0;
  }
  .muted {
    color: var(--muted-foreground, inherit);
  }
  .grid {
    display: grid;
    gap: 1.5rem;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  @media (min-width: 40rem) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .grid > li {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  /* A radius token is the PRODUCT's decision and may legitimately be 999px —
     a pill design system. On a small control that is the intent; on a large
     surface the same token turns a card into a circle, which is exactly what
     shipped on a press page. So a large surface follows the product's scale
     only up to a cap, and small controls (buttons, menu rows) still take it
     whole. */
  .logo-frame {
    display: flex;
    align-items: center;
    justify-content: center;
    aspect-ratio: 3 / 2;
    padding: 2rem;
    border: 1px solid var(--border, currentColor);
    border-radius: min(var(--radius-md, 6px), 12px);
  }
  .logo-frame img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 1rem;
  }
  .cap {
    font-size: 0.875rem;
    color: var(--muted-foreground, inherit);
  }
  .dl {
    flex-shrink: 0;
    padding: 0.375rem 0.75rem;
    border: 1px solid var(--border, currentColor);
    border-radius: var(--radius-sm, 3px);
    font-size: 0.875rem;
    font-weight: 500;
    color: inherit;
    text-decoration: none;
  }
  .facts {
    display: grid;
    gap: 0 2rem;
    margin: 0;
    padding: 1.5rem;
    border: 1px solid var(--border, currentColor);
    border-radius: min(var(--radius-md, 6px), 12px);
  }
  @media (min-width: 40rem) {
    .facts {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .fact {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 0.125rem;
    padding: 0.75rem 0;
    border-bottom: 1px solid var(--border, rgb(0 0 0 / 0.1));
  }
  .fact dt {
    font-size: 0.875rem;
    color: var(--muted-foreground, inherit);
  }
  .fact dd {
    margin: 0;
    overflow-wrap: break-word;
  }
  .boiler {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .boiler h3 {
    font-size: 0.875rem;
    font-weight: 600;
  }
  textarea {
    width: 100%;
    padding: 1rem;
    resize: none;
    background: transparent;
    color: inherit;
    font: inherit;
    border: 1px solid var(--border, currentColor);
    border-radius: min(var(--radius-md, 6px), 12px);
    field-sizing: content;
  }
  .story {
    display: flex;
    height: 100%;
    flex-direction: column;
    gap: 0.5rem;
    padding: 1rem;
    border: 1px solid var(--border, currentColor);
    border-radius: min(var(--radius-md, 6px), 12px);
    color: inherit;
    text-decoration: none;
    transition: border-color 120ms ease;
  }
  .story:hover {
    border-color: var(--primary, currentColor);
  }
  .cover {
    overflow: hidden;
    aspect-ratio: 16 / 9;
    border-radius: var(--radius-sm, 3px);
    margin-bottom: 0.5rem;
  }
  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0 0.75rem;
    font-size: 0.875rem;
    color: var(--muted-foreground, inherit);
  }
</style>
