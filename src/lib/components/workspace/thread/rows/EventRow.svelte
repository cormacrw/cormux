<script lang="ts">
  import {
    formatThreadTime,
    formatThreadTimeTitle,
  } from '$lib/thread/thread-time'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import type { RowContext } from './context'
  import { ICONS } from './icons'
  import RowShell from './RowShell.svelte'
  import { stepIcon } from './step-icons'

  let {
    item,
    ctx,
  }: { item: Extract<TimelineItem, { kind: 'event' }>; ctx: RowContext } =
    $props()

  const Icon = $derived(ICONS[stepIcon(item.step)])
  const clock = $derived(formatThreadTime(item.atMs))
  // Titles wrap branch names in backticks; odd parts render as code.
  const parts = $derived(
    item.title
      .split('`')
      .map((text, index) => ({ text, code: index % 2 === 1 })),
  )
</script>

<RowShell id={item.id} {ctx} pop>
  <div
    class="flex items-center gap-3 py-1 text-xs text-foreground/80"
    title={[item.detail, formatThreadTimeTitle(item.atMs)]
      .filter(Boolean)
      .join('\n')}
  >
    <span class="h-px min-w-6 flex-1 bg-border" aria-hidden="true"></span>
    <span class="flex min-w-0 items-center gap-1.5 font-medium">
      <Icon class="size-3 shrink-0" aria-hidden="true" />
      <span class="truncate"
        >{#each parts as part, index (index)}{#if part.code}<span
              class="font-mono">{part.text}</span
            >{:else}{part.text}{/if}{/each}</span
      >
      {#if clock}
        <span class="shrink-0 font-normal text-muted-foreground">· {clock}</span
        >
      {/if}
    </span>
    <span class="h-px min-w-6 flex-1 bg-border" aria-hidden="true"></span>
  </div>
</RowShell>
