<!--
  A fold whose content isn't rendered while it's closed: a thought re-renders its markdown on
  every chunk otherwise. Whether it's open is kept for the thread rather than by this instance:
  a long thread renders each row in its own instance and unmounts rows off screen, so a fold
  opened there would close on scrolling back. Find opens it by adding its id to `openFolds`.
-->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { RowContext } from './context'

  let {
    id,
    ctx,
    class: className,
    summaryClass,
    odId,
    summary,
    children,
  }: {
    id: string
    ctx: RowContext
    class?: string
    summaryClass: string
    odId?: string
    summary: Snippet
    children: Snippet
  } = $props()

  const open = $derived(ctx.openFolds.has(id))

  function toggled(event: Event) {
    if ((event.currentTarget as HTMLDetailsElement).open) ctx.openFolds.add(id)
    else ctx.openFolds.delete(id)
  }
</script>

<details
  class={className}
  data-od-id={odId}
  data-lazy-fold
  {open}
  ontoggle={toggled}
>
  <summary class={summaryClass}>{@render summary()}</summary>
  {#if open}
    {@render children()}
  {/if}
</details>
