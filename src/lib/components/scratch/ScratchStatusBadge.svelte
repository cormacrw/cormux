<script lang="ts">
  import { cn } from '$lib/utils'
  import { settings } from '$lib/state'
  import {
    SCRATCH_STATUS_LABEL,
    type ScratchStatus,
  } from '$lib/state/scratches.svelte'

  let { status, class: className }: { status: ScratchStatus; class?: string } =
    $props()

  // Needs attention shares Paused's amber; only the words differ.
  const amber = $derived(status === 'needsAttention' || status === 'paused')
</script>

<span
  class={cn(
    'inline-flex h-7 shrink-0 items-center gap-2 rounded-full px-3 text-[13px] font-bold text-cocoa shadow-[inset_0_-2px_0_rgb(0_0_0/0.1)]',
    status === 'working' && 'bg-leaf',
    amber && 'bg-marigold',
    status === 'idle' && 'bg-secondary text-secondary-foreground',
    className,
  )}
>
  {#if status === 'working'}
    <span
      class={cn('bead size-2 bg-[#fff6e6]', !settings.reduceMotion && 'animate-pulse')}
      aria-hidden="true"
    ></span>
  {/if}
  {SCRATCH_STATUS_LABEL[status]}
</span>
