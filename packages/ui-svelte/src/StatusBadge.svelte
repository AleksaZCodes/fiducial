<script lang="ts" generics="T, E">
  import type { Result } from "@fiducial/headless";

  interface Props {
    result: Result<T, E>;
    renderOk?: (value: T) => string;
    renderErr?: (error: E) => string;
  }

  let {
    result,
    renderOk = String,
    renderErr = String,
  }: Props = $props();
</script>

{#if result.ok}
  <span class="badge badge-ok">✓ {renderOk(result.value)}</span>
{:else}
  <span class="badge badge-err">✗ {renderErr(result.error)}</span>
{/if}

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 500;
  }

  .badge-ok {
    background: var(--secondary);
    color: var(--secondary-foreground);
  }

  .badge-err {
    background: color-mix(in oklch, var(--destructive) 15%, transparent);
    color: var(--destructive);
  }
</style>
