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

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  class={["fid-dialog", className].filter(Boolean).join(" ")}
  onclose={handleClose}
  onclick={handleBackdropClick}
>
  <div onclick={(e) => e.stopPropagation()}>
    {@render children?.()}
  </div>
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
