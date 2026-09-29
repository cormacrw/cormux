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
    'inline-flex h-7 shrink-0 items-center gap-2 rounded-full border px-3 text-xs font-medium',
    status === 'working' &&
      'border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300',
    amber &&
      'border-amber-500/40 bg-amber-500/10 text-amber-800 dark:text-amber-200',
    status === 'idle' && 'border-border bg-muted/40 text-muted-foreground',
    className,
  )}
>
  <span
    class={cn(
      'size-2 rounded-full',
      status === 'working' && 'bg-emerald-500',
      status === 'working' && !settings.reduceMotion && 'animate-pulse',
      amber && 'bg-amber-500',
      status === 'idle' && 'bg-muted-foreground/60',
    )}
    aria-hidden="true"
  ></span>
  {SCRATCH_STATUS_LABEL[status]}
</span>
