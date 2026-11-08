<script lang="ts">
  import {
    formatThreadTime,
    formatThreadTimeTitle,
  } from '$lib/thread/thread-time'
  import type { TimelineRow } from '$lib/thread/timeline-types'
  import type { RowContext } from './context'
  import { popIn } from './pop-in'

  let {
    row,
    ctx,
  }: { row: Extract<TimelineRow, { kind: 'speaker' }>; ctx: RowContext } =
    $props()

  const clock = $derived(formatThreadTime(row.atMs))
</script>

<li
  class="flex items-center gap-2 py-1 text-xs text-muted-foreground"
  aria-hidden="true"
  use:ctx.trackRow={row.id}
  use:popIn={{ entries: ctx.entries, key: row.id }}
>
  {#if row.role}
    <span class="font-medium text-foreground">{row.role}</span>
    <span>{row.engine}</span>
  {:else}
    <span class="font-medium text-foreground">{row.engine}</span>
  {/if}
  {#if clock}
    <span
      class="text-muted-foreground/80"
      title={formatThreadTimeTitle(row.atMs)}>· {clock}</span
    >
  {/if}
</li>
