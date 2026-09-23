<script lang="ts">
  /**
   * One image with its caption and a download button.
   *
   * Its React twin is `press/for/web-next/.../media-figure.tsx`. Change one,
   * change both.
   *
   * Every image on a document page goes through this: press gallery items,
   * story and post covers, and images inside a post's body. One component,
   * because a press asset is used by someone else, and a journalist lifting an
   * image needs the same three things wherever they find it — the picture,
   * what it shows, and a way to save the original rather than a screenshot of
   * it.
   *
   * The download goes through `?download`, which makes the media route send
   * `Content-Disposition: attachment`. A bare `download` attribute is ignored
   * by some browsers for anything they can display, which opens the image in a
   * tab instead of saving it — the one thing the button exists to prevent.
   */
  import { mediaUrl } from "../media";

  let {
    mediaKey,
    alt,
    caption,
    downloadLabel,
    aspect = "3 / 2",
    fit = "cover",
    priority = false,
    /**
     * The surface to put behind the image.
     *
     * It matters for anything with transparency, and it is the fix for a bug
     * that shipped: a logo drawn in light colours, dropped on the ambient page
     * background, is invisible. A mark's file name usually names the surface
     * it is drawn *for*, not its own colour, so read the fills before deciding
     * — `mark-ink.svg` being drawn in pale aqua is not a contradiction, it is
     * the mark for an ink-coloured field.
     */
    background,
  }: {
    mediaKey: string;
    alt: string;
    caption?: string;
    /** Omit to show the image with no download button (e.g. a card thumbnail). */
    downloadLabel?: string;
    aspect?: string;
    /**
     * `contain` for an asset whose whole frame matters — a logo, an icon, a
     * diagram — where cropping would misrepresent the file being downloaded.
     */
    fit?: "cover" | "contain";
    priority?: boolean;
    background?: string;
  } = $props();

  const src = $derived(mediaUrl(mediaKey));
</script>

<figure class="fig">
  <div class="frame" style:aspect-ratio={aspect} style:background>
    <img
      {src}
      {alt}
      loading={priority ? "eager" : "lazy"}
      decoding="async"
      style:object-fit={fit}
    />
  </div>
  {#if caption || downloadLabel}
    <figcaption>
      <span class="cap">{caption ?? ""}</span>
      {#if downloadLabel}
        <a class="dl" href={`${src}?download`}>{downloadLabel}</a>
      {/if}
    </figcaption>
  {/if}
</figure>

<style>
  .fig {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    margin: 0;
  }
  /* A radius token is the PRODUCT's decision and may legitimately be 999px —
     a pill design system. On a small control that is the intent; on a large
     surface the same token turns a card into a circle, which is exactly what
     shipped on a press page. So a large surface follows the product's scale
     only up to a cap, and small controls (buttons, menu rows) still take it
     whole. */
  .frame {
    overflow: hidden;
    border: 1px solid var(--border, currentColor);
    border-radius: min(var(--radius-md, 6px), 12px);
  }
  .frame img {
    width: 100%;
    height: 100%;
    display: block;
  }
  figcaption {
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
</style>
