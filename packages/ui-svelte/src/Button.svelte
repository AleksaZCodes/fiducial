<script lang="ts">
  interface Props {
    variant?: "primary" | "secondary" | "ghost";
    disabled?: boolean;
    onclick?: () => void;
    children?: import("svelte").Snippet;
  }

  let {
    variant = "primary",
    disabled = false,
    onclick,
    children,
  }: Props = $props();

  const variantClass = {
    primary:   "btn-primary",
    secondary: "btn-secondary",
    ghost:     "btn-ghost",
  } as const;
</script>

<button class="btn {variantClass[variant]}" {disabled} {onclick}>
  {@render children?.()}
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.5rem 1rem;
    border-radius: var(--radius);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s;
    border: none;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .btn-secondary {
    background: var(--secondary);
    color: var(--secondary-foreground);
    border: 1px solid var(--border);
  }

  .btn-ghost {
    background: transparent;
    color: var(--foreground);
  }
</style>
