<script lang="ts">
  interface Props {
    variant?: "primary" | "secondary" | "ghost" | "destructive";
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    onclick?: () => void;
    type?: "button" | "submit" | "reset";
    class?: string;
    children?: import("svelte").Snippet;
  }

  let {
    variant = "primary",
    size = "md",
    disabled = false,
    onclick,
    type = "button",
    class: className,
    children,
  }: Props = $props();
</script>

<button
  {type}
  {disabled}
  {onclick}
  data-variant={variant}
  data-size={size}
  class={["fid-btn", className].filter(Boolean).join(" ")}
>
  {@render children?.()}
</button>

<style>
  :global(.fid-btn) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    border-radius: var(--radius);
    font-size: 0.875rem;
    font-weight: 500;
    line-height: 1;
    cursor: pointer;
    border: 1px solid transparent;
    transition: opacity 0.15s, background 0.15s;
    white-space: nowrap;
    user-select: none;
  }
  :global(.fid-btn:disabled) { opacity: 0.5; cursor: not-allowed; pointer-events: none; }
  :global(.fid-btn[data-size="sm"]) { padding: 0.375rem 0.75rem; font-size: 0.8125rem; }
  :global(.fid-btn[data-size="md"]) { padding: 0.5rem 1rem; }
  :global(.fid-btn[data-size="lg"]) { padding: 0.625rem 1.25rem; font-size: 1rem; }
  :global(.fid-btn[data-variant="primary"]) { background: var(--primary); color: var(--primary-foreground); }
  :global(.fid-btn[data-variant="primary"]:hover:not(:disabled)) { opacity: 0.9; }
  :global(.fid-btn[data-variant="secondary"]) { background: var(--secondary); color: var(--secondary-foreground); border-color: var(--border); }
  :global(.fid-btn[data-variant="secondary"]:hover:not(:disabled)) { background: var(--accent); }
  :global(.fid-btn[data-variant="ghost"]) { background: transparent; color: var(--foreground); }
  :global(.fid-btn[data-variant="ghost"]:hover:not(:disabled)) { background: var(--accent); color: var(--accent-foreground); }
  :global(.fid-btn[data-variant="destructive"]) { background: var(--destructive); color: var(--primary-foreground); }
  :global(.fid-btn[data-variant="destructive"]:hover:not(:disabled)) { opacity: 0.9; }
</style>
