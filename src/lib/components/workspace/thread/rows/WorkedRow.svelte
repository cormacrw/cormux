<script lang="ts">
  import Clock from '@lucide/svelte/icons/clock'
  import { formatThreadTimeTitle } from '$lib/thread/thread-time'
  import { workedForLabel } from '$lib/thread/timeline/step-text'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import type { RowContext } from './context'
  import RowShell from './RowShell.svelte'

  let {
    item,
    ctx,
  }: { item: Extract<TimelineItem, { kind: 'worked' }>; ctx: RowContext } =
    $props()
</script>

<RowShell id={item.id} {ctx} pop>
  <div class="space-y-1.5">
    <ul class="space-y-1.5">
      <li
        class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
        title={formatThreadTimeTitle(item.atMs)}
      >
        <Clock
          class="size-3 shrink-0 text-muted-foreground/80"
          aria-hidden="true"
        />
        <span class="shrink-0"
          >{workedForLabel(item.startedAtMs, item.atMs)}</span
        >
      </li>
    </ul>
  </div>
</RowShell>
