<script lang="ts">
  import { onDestroy } from 'svelte'
  import Check from '@lucide/svelte/icons/check'
  import Copy from '@lucide/svelte/icons/copy'
  import { CopyFeedback } from '$lib/primitives/copy-feedback.svelte'
  import {
    formatThreadTime,
    formatThreadTimeTitle,
  } from '$lib/thread/thread-time'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import type { RowContext } from './context'
  import RowShell from './RowShell.svelte'

  let {
    item,
    ctx,
  }: { item: Extract<TimelineItem, { kind: 'user' }>; ctx: RowContext } =
    $props()

  const clock = $derived(formatThreadTime(item.atMs))
  const clipboard = new CopyFeedback()
  onDestroy(() => clipboard.dispose())
</script>

<RowShell
  id={item.id}
  {ctx}
  pop
  origin="right bottom"
  class="flex flex-col items-end gap-1"
>
  <div class="group relative max-w-[min(100%,34rem)]">
    <div
      class="rounded-[18px] rounded-br-md user-bubble border border-[color-mix(in_oklch,var(--primary)_60%,white_10%)] px-3.5 py-2 text-sm leading-5"
    >
      <p class="whitespace-pre-wrap">{item.text}</p>
    </div>
    <button
      type="button"
      class="absolute top-1 right-1 inline-flex size-5 items-center justify-center rounded text-primary-foreground/70 opacity-0 transition hover:text-primary-foreground focus-visible:opacity-100 group-hover:opacity-100"
      aria-label="Copy message"
      onclick={() => clipboard.copy(item.text)}
    >
      {#if clipboard.copied}
        <Check class="size-3" aria-hidden="true" />
      {:else}
        <Copy class="size-3" aria-hidden="true" />
      {/if}
    </button>
  </div>
  <p
    class="text-[11px] text-muted-foreground"
    title={formatThreadTimeTitle(item.atMs)}
  >
    You{clock ? ` · ${clock}` : ''}
  </p>
</RowShell>
