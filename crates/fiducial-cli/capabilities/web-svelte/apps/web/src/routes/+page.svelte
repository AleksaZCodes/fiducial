<script lang="ts">
import type { Result } from "@fiducial/headless";

// Typed product state using headless Result — same pattern as web-next.
// `as`, not an annotation: annotated, TypeScript narrows the union on
// assignment and `status.error` below no longer type-checks.
const status = { ok: true, value: "{{name}}" } as Result<string, string>;
</script>

<main>
  <h1>{status.ok ? status.value : "Error"}</h1>
  <p>
    Built on <a href="https://github.com/AleksaZCodes/fiducial">Fiducial</a>
    {{version}}.
  </p>
  {#if !status.ok}
    <p class="error">{status.error}</p>
  {/if}
</main>

<style>
  main {
    padding: 2rem;
    max-width: 640px;
    margin: 0 auto;
  }

  h1 {
    color: var(--primary);
    margin-bottom: 0.5rem;
  }

  p {
    color: var(--muted-foreground);
  }

  a {
    color: var(--primary);
  }

  .error {
    color: var(--destructive);
  }
</style>
