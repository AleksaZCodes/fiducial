<script lang="ts">
  interface Props {
    variant?: "default" | "secondary" | "destructive" | "outline";
    class?: string;
    children?: import("svelte").Snippet;
  }
  let { variant = "default", class: className, children }: Props = $props();
</script>

<span
  data-variant={variant}
  class={["fid-badge", className].filter(Boolean).join(" ")}
>
  {@render children?.()}
</span>

<style>
  :global(.fid-badge) {
    display: inline-flex;
    align-items: center;
    border-radius: 9999px;
    padding: 0.125rem 0.625rem;
    font-size: 0.75rem;
    font-weight: 500;
    line-height: 1.5;
    border: 1px solid transparent;
  }
  :global(.fid-badge[data-variant="default"]) { background: var(--primary); color: var(--primary-foreground); }
  :global(.fid-badge[data-variant="secondary"]) { background: var(--secondary); color: var(--secondary-foreground); }
  :global(.fid-badge[data-variant="destructive"]) { background: var(--destructive); color: var(--primary-foreground); }
  :global(.fid-badge[data-variant="outline"]) { background: transparent; color: var(--foreground); border-color: var(--border); }
</style>
