<script lang="ts">
  import Check from '@lucide/svelte/icons/check'
  import { cn } from '$lib/utils'
  import type { WeekItem } from '$lib/week/items'

  // Ticks a task off, or back on. Sprint tasks are only marked done here, never in ClickUp.
  let { item, onToggle }: { item: WeekItem; onToggle: () => void } = $props()
</script>

<button
  type="button"
  class={cn(
    'inline-flex h-8 items-center gap-1.5 rounded-md border px-2.5 text-xs font-medium outline-none transition-[transform,background-color] duration-150 focus-visible:ring-3 focus-visible:ring-ring/50 active:scale-[0.98] disabled:opacity-60',
    item.done
      ? 'border-success/50 bg-success/15 text-success'
      : 'border-border hover:bg-muted',
  )}
  aria-pressed={item.done}
  disabled={item.doneInClickup}
  title={item.doneInClickup ? 'Done in ClickUp' : undefined}
  data-od-id="week-toggle-done"
  onclick={onToggle}
>
  <Check class="size-3.5" strokeWidth={2.6} aria-hidden="true" />
  {item.doneInClickup
    ? 'Done in ClickUp'
    : item.done
      ? 'Completed · mark not done'
      : 'Mark complete'}
</button>
