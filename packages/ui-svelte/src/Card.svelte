<script lang="ts">
  interface Props {
    class?: string;
    children?: import("svelte").Snippet;
  }
  let { class: className, children }: Props = $props();
</script>

<div class={["fid-card", className].filter(Boolean).join(" ")}>
  {@render children?.()}
</div>

<style>
  :global(.fid-card) {
    background: var(--card);
    color: var(--card-foreground);
    border: 1px solid var(--border);
    border-radius: calc(var(--radius) + 2px);
    box-shadow: 0 1px 3px oklch(0 0 0 / 8%);
  }
</style>
