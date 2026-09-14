<script lang="ts">
  interface Props {
    open?: boolean;
    onclose?: () => void;
    class?: string;
    children?: import("svelte").Snippet;
  }

  let { open = false, onclose, class: className, children }: Props = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    if (!dialog) return;
    if (open) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  });

  function handleClose() {
    onclose?.();
  }

  // A click on a `<dialog>` that opened with `showModal()` reports coordinates
  // in the backdrop when it lands outside the dialog's own box. Hit-testing the
  // rect is therefore enough to tell backdrop from content — no stopPropagation
  // wrapper is needed, and the one that used to live here was both redundant
  // and an unlabelled interactive `<div>`.
  function handleBackdropClick(e: MouseEvent) {
    const rect = dialog.getBoundingClientRect();
    const outside =
      e.clientX < rect.left || e.clientX > rect.right ||
      e.clientY < rect.top  || e.clientY > rect.bottom;
    if (outside) dialog.close();
  }
</script>

<!-- The native `<dialog>` element is the interactive control here: `showModal()`
     supplies the focus trap, the Escape binding and the backdrop. The click
     handler only distinguishes backdrop from content, so no extra ARIA role or
     key handler applies. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<dialog
  bind:this={dialog}
  class={["fid-dialog", className].filter(Boolean).join(" ")}
  onclose={handleClose}
  onclick={handleBackdropClick}
>
  {@render children?.()}
</dialog>

<style>
  :global(.fid-dialog) {
    background: var(--card);
    color: var(--card-foreground);
    border: 1px solid var(--border);
    border-radius: calc(var(--radius) + 2px);
    box-shadow: 0 8px 32px oklch(0 0 0 / 20%);
    padding: 1.5rem;
    width: min(90vw, 28rem);
    max-height: 85vh;
    overflow-y: auto;
  }

  :global(.fid-dialog::backdrop) {
    background: oklch(0 0 0 / 40%);
  }
</style>
