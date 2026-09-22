<script lang="ts">
  /**
   * The nav links on a narrow screen, in a sheet that slides in from the right.
   *
   * ## Why a dialog and not a disclosure
   *
   * A row that pushes the header down is simpler and wrong: focus stays behind
   * it, Escape does nothing, the page underneath keeps scrolling, and nothing
   * returns focus to the button when it closes. This is a modal dialog, so all
   * four are handled by the primitive rather than by four listeners somebody
   * has to remember. bits-ui is what shadcn-svelte builds its Sheet from;
   * using it directly is the same behaviour without the copy-in step.
   *
   * The version this replaces was `let open = $state(false)` with a
   * `<svelte:window onkeydown>` for Escape and an overlay `<div onclick>`. It
   * looked equivalent and had none of the focus behaviour.
   *
   * ## What is the product's, and what is not
   *
   * This component owns the sheet: how it opens, how it closes, what a screen
   * reader hears, and where focus goes. It owns no words and no routes —
   * `links` and both labels come in, because which pages a product has is the
   * product's fact and translating them is the catalog's job. That line is
   * deliberate: a menu that hard-codes four `href`s is a menu every product
   * has to fork, and one that takes twenty props to avoid it is worse.
   *
   * Composition also stays out here. There is no `SiteNav` in the platform on
   * purpose: arranging a wordmark, links, a language picker and a call to
   * action is exactly the decision each product makes differently, and the
   * pieces being separate is what lets it.
   */
  import { Dialog } from "bits-ui";

  let {
    links,
    openLabel,
    closeLabel,
  }: {
    links: readonly { href: string; label: string }[];
    openLabel: string;
    closeLabel: string;
  } = $props();

  let open = $state(false);
</script>

<!-- `display: contents`, so this wrapper adds no box to the parent's flex or
     grid — the trigger sits exactly where a bare <button> would. -->
<div class="fid-mobile-only">
  <Dialog.Root bind:open>
    <Dialog.Trigger class="fid-burger" aria-label={openLabel}>
      <!-- Drawn rather than pulled from an icon set: two paths beat a
           dependency, and square ends match an unrounded design system. -->
      <svg viewBox="0 0 16 16" aria-hidden="true" class="fid-burger-icon">
        <path d="M2 4 H14 M2 8 H14 M2 12 H14" stroke="currentColor" stroke-width="1.75" />
      </svg>
    </Dialog.Trigger>

    <Dialog.Portal>
      <Dialog.Overlay class="fid-sheet-overlay" />
      <Dialog.Content class="fid-sheet">
        <!-- The accessible name. A dialog without one is announced as
             "dialog" and nothing else; on screen a heading reading "Menu"
             above four links labels something already obvious, so it is
             visually hidden rather than dropped. -->
        <Dialog.Title class="fid-sr-only">{openLabel}</Dialog.Title>
        <Dialog.Close class="fid-sheet-close" aria-label={closeLabel}>
          <svg viewBox="0 0 16 16" aria-hidden="true" class="fid-burger-icon">
            <path d="M3 3 L13 13 M13 3 L3 13" stroke="currentColor" stroke-width="1.75" />
          </svg>
        </Dialog.Close>
        <ul class="fid-sheet-list">
          {#each links as l (l.href)}
            <li>
              <!-- Closing on click matters most for a fragment link like
                   `/#contact`: it navigates without a page load, so nothing
                   else would ever dismiss the sheet and the reader is left
                   staring at a menu over the section they asked for. -->
              <a class="fid-sheet-link" href={l.href} onclick={() => (open = false)}>{l.label}</a>
            </li>
          {/each}
        </ul>
      </Dialog.Content>
    </Dialog.Portal>
  </Dialog.Root>
</div>

<!-- `:global`, because the sheet is portalled out of this subtree and a scoped
     class would style nothing. `fid-` prefixes keep these off a product's own
     names. -->
<style>
  .fid-mobile-only {
    display: contents;
  }
  :global(.fid-burger) {
    display: none;
    padding: 0.375rem;
    border: 1px solid var(--border, currentColor);
    border-radius: var(--radius-sm, 3px);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  /* The breakpoint is a CSS rule, not a prop: a media query cannot read a
     custom property, so a `breakpoint="60rem"` prop would have silently done
     nothing. A product that wants a different one overrides `.fid-burger`
     in its own stylesheet, where the query belongs. */
  @media (max-width: 48rem) {
    :global(.fid-burger) {
      display: inline-flex;
    }
  }
  :global(.fid-burger-icon) {
    width: 1rem;
    height: 1rem;
    display: block;
  }
  :global(.fid-sheet-overlay) {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.6);
    z-index: 70;
  }
  :global(.fid-sheet) {
    position: fixed;
    inset-block: 0;
    inset-inline-end: 0;
    width: min(20rem, 85vw);
    padding: 1.5rem;
    background: var(--background, #fff);
    color: var(--foreground, inherit);
    box-shadow: -8px 0 24px rgb(0 0 0 / 0.25);
    z-index: 80;
  }
  :global(.fid-sheet-close) {
    display: block;
    margin-inline-start: auto;
    padding: 0.5rem;
    border: none;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  :global(.fid-sheet-list) {
    display: flex;
    flex-direction: column;
    margin: 1.5rem 0 0;
    padding: 0;
    list-style: none;
  }
  :global(.fid-sheet-link) {
    display: block;
    padding: 0.875rem 0.25rem;
    color: inherit;
    text-decoration: none;
    font-size: 1.1rem;
    border-bottom: 1px solid var(--border, rgb(0 0 0 / 0.1));
  }
  :global(.fid-sr-only) {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }
</style>
