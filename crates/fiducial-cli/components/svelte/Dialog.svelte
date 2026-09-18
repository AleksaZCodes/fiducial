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

  function handleBackdropClick(e: MouseEvent) {
    const rect = dialog.getBoundingClientRect();
    const outside =
      e.clientX < rect.left || e.clientX > rect.right ||
      e.clientY < rect.top  || e.clientY > rect.bottom;
    if (outside) dialog.close();
  }
</script>

<!-- The click handler lives on <dialog> itself because that is what a click on
     the ::backdrop targets — the backdrop is a pseudo-element and cannot carry
     a listener of its own. `handleBackdropClick` then compares the pointer
     against the dialog's own bounding rect, so a click on the content is
     already excluded by geometry.

     There used to be an inner `<div onclick={stopPropagation}>` doing that
     exclusion a second time. It was redundant, and it put a click handler on a
     non-interactive element with no role and no keyboard equivalent — which is
     what `--fail-on-warnings` was reporting. Closing by keyboard is Escape,
     which <dialog> handles natively. -->
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
