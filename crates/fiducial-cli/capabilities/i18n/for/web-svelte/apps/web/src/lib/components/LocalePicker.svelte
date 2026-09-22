<script lang="ts">
  /**
   * The language picker — the Svelte half of the platform's locale switcher.
   *
   * Its React twin is `for/web-next/apps/web/src/components/locale-picker.tsx`
   * and every decision below is that file's, restated in another framework.
   * Read this list before changing either: each item is a bug that shipped.
   *
   * **A list, not a toggle.** It used to be a single link to "the other
   * locale". That works for exactly two languages and is silently wrong at
   * three.
   *
   * **Every locale names itself.** A language is listed under its endonym —
   * `Srpski`, not `Serbian` — read from that locale's own catalog
   * (`locale.name`). A picker exists for someone who cannot read the page
   * they are looking at, so translating the list into the language they are
   * trying to leave is the one arrangement guaranteed to be useless. There is
   * no table of languages here and no branch on a locale code: dropping in
   * `messages/de.json` with a `locale` block adds German and nothing else has
   * to be touched, which is the whole test of whether an i18n layer is real.
   *
   * **The flag is decoration and is marked as such.** A flag is a country and
   * a locale is a language; Serbian is not confined to Serbia. The flag is
   * `aria-hidden`, the endonym is the label, so a screen reader announces the
   * language and never a nation. A region with no flag in the set renders as
   * no flag rather than as a broken image — the name was always what carried
   * the meaning.
   *
   * **Real links.** Each row is an `<a href>`, so it is middle-clickable,
   * copyable and crawlable, with `hreflang` stating the relationship. A
   * switcher built from click handlers is invisible to a search engine and to
   * anyone opening it in a new tab.
   *
   * **bits-ui, not a hand-rolled dropdown.** The version this replaces was
   * `let open = $state(false)` plus `<svelte:window onclick={close}>`, which
   * has no focus trap, no roving tabindex, no Escape handling, no
   * `aria-activedescendant`, and closes on any click anywhere including the
   * one opening it. bits-ui is what shadcn-svelte is built on; using it
   * directly is the same components without the copy-in step.
   */
  import RS from "country-flag-icons/string/3x2/RS";
  import GB from "country-flag-icons/string/3x2/GB";
  import { DropdownMenu } from "bits-ui";
  import { type Locale, locales, messages } from "../../generated/messages";
  import { queryLocaleHref } from "../locale-href";

  let {
    locale,
    variant = "full",
    /**
     * The page the picker is on. Passed in rather than read from `$app/state`
     * so this component has no route dependency and can be rendered in a test
     * or a story.
     */
    url,
    /**
     * Where a language leads. Defaults to `?lang=`, which needs no routing.
     * A product with locale-prefixed routes passes `prefixLocaleHref`.
     */
    href = (target: Locale) => queryLocaleHref(url, target),
  }: {
    locale: Locale;
    variant?: "full" | "icon";
    url: URL | string;
    href?: (target: Locale) => string;
  } = $props();

  /**
   * Flags as plain SVG strings (`country-flag-icons/string/3x2/<REGION>`)
   * rather than the package's React components — this is Svelte, and a flag
   * is a fixed 3:2 mark, not something that needs per-framework logic.
   *
   * Extend this map when a product declares a locale whose region is not
   * here; an absent region renders no flag, which is a deliberate
   * non-failure.
   */
  const flags: Record<string, string> = { RS, GB };
  const flagFor = (l: Locale) => flags[messages[l]["locale.region"]];
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger
    class="fid-locale-trigger"
    data-variant={variant}
    aria-label={messages[locale]["locale.switch"]}
    title={variant === "icon" ? messages[locale]["locale.name"] : undefined}
  >
    {#if flagFor(locale)}
      <span class="fid-flag" aria-hidden="true">{@html flagFor(locale)}</span>
    {/if}
    {#if variant === "full"}
      <span>{messages[locale]["locale.name"]}</span>
      <svg viewBox="0 0 16 16" class="fid-chevron" aria-hidden="true">
        <path d="M4 6 L8 10 L12 6" fill="none" stroke="currentColor" stroke-width="1.5" />
      </svg>
    {/if}
  </DropdownMenu.Trigger>

  <DropdownMenu.Content class="fid-locale-menu" align="end" sideOffset={6}>
    {#each locales as l (l)}
      <DropdownMenu.Item>
        {#snippet child({ props })}
          <!-- `child` is bits-ui's delegation snippet — React's `asChild`.
               It keeps every menu behaviour (roving focus, typeahead, Escape,
               close-on-select) while the rendered element stays a real link. -->
          <a
            {...props}
            class="fid-locale-item"
            href={href(l)}
            hreflang={l}
            lang={l}
            aria-current={l === locale ? "true" : undefined}
          >
            {#if flagFor(l)}
              <span class="fid-flag" aria-hidden="true">{@html flagFor(l)}</span>
            {/if}
            <span class="fid-locale-name">{messages[l]["locale.name"]}</span>
            {#if l === locale}
              <svg viewBox="0 0 16 16" class="fid-check" aria-hidden="true">
                <path d="M3 8.5 L6.5 12 L13 4" fill="none" stroke="currentColor" stroke-width="1.75" />
              </svg>
            {/if}
          </a>
        {/snippet}
      </DropdownMenu.Item>
    {/each}
  </DropdownMenu.Content>
</DropdownMenu.Root>

<!-- `:global` throughout, and not by accident: bits-ui renders the menu into a
     floating layer outside this component's DOM subtree, where Svelte's scoped
     class never reaches. Scoped styles here would compile, apply to nothing,
     and leave an unstyled menu — so the classes are prefixed `fid-` instead,
     which is what keeps them from colliding with a product's own. -->
<style>
  :global(.fid-locale-trigger) {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.625rem;
    border: 1px solid var(--border, currentColor);
    border-radius: var(--radius-sm, 3px);
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  :global(.fid-locale-trigger[data-variant="icon"]) {
    padding: 0.375rem;
  }
  :global(.fid-flag) {
    display: inline-flex;
    width: 1.3125rem;
    height: 0.875rem;
    flex-shrink: 0;
    overflow: hidden;
    border-radius: 1px;
  }
  :global(.fid-flag svg) {
    width: 100%;
    height: 100%;
    display: block;
  }
  :global(.fid-chevron),
  :global(.fid-check) {
    width: 0.875rem;
    height: 0.875rem;
    opacity: 0.6;
  }
  :global(.fid-locale-menu) {
    min-width: 10rem;
    padding: 0.25rem;
    border: 1px solid var(--border, currentColor);
    border-radius: var(--radius-md, 4px);
    background: var(--popover, var(--background, #fff));
    color: var(--popover-foreground, var(--foreground, inherit));
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.2);
    z-index: 60;
  }
  :global(.fid-locale-item) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.625rem;
    border-radius: var(--radius-sm, 3px);
    color: inherit;
    text-decoration: none;
    cursor: pointer;
  }
  :global(.fid-locale-name) {
    flex: 1;
  }
  /* `data-highlighted` is bits-ui's own attribute for the focused item, so
     keyboard and pointer highlight the same way without a second rule. */
  :global(.fid-locale-item:hover),
  :global(.fid-locale-item[data-highlighted]) {
    background: var(--accent, rgb(0 0 0 / 0.06));
    color: var(--accent-foreground, inherit);
  }
</style>
