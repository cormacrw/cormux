<script lang="ts">
  import type { ClickupTask } from '$lib/ipc/bindings'
  import { formatPoints } from '$lib/clickup/board'
  import { cn } from '$lib/utils'
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert'

  let {
    task,
    selected = false,
    dragging = false,
    ghost = false,
    onSelect,
    onPointerDown,
  }: {
    task: ClickupTask
    selected?: boolean
    /** The card is being dragged; this copy stays behind as a placeholder. */
    dragging?: boolean
    /** The copy that follows the pointer. */
    ghost?: boolean
    onSelect?: () => void
    onPointerDown?: (event: PointerEvent) => void
  } = $props()

  const unpointed = $derived(task.points == null)
</script>

<button
  type="button"
  class={cn(
    'group/card relative flex w-full touch-none select-none flex-col gap-2 overflow-hidden rounded-lg border bg-card p-3 text-left text-card-foreground shadow-xs outline-none transition-[border-color,background-color,opacity,box-shadow] focus-visible:ring-3 focus-visible:ring-ring/50',
    unpointed
      ? 'border-warning/60 bg-warning/[0.06] [@media(hover:hover)_and_(pointer:fine)]:hover:border-warning'
      : 'border-border [@media(hover:hover)_and_(pointer:fine)]:hover:border-foreground/25',
    selected && 'border-primary ring-2 ring-primary/30',
    dragging && 'opacity-40',
    ghost && 'rotate-[1.5deg] cursor-grabbing shadow-xl',
  )}
  aria-pressed={ghost ? undefined : selected}
  aria-label="{task.customId ? `${task.customId} ` : ''}{task.name}{unpointed
    ? ', no sprint points'
    : `, ${formatPoints(task.points!)} points`}"
  data-task-id={ghost ? undefined : task.id}
  data-unpointed={unpointed || undefined}
  tabindex={ghost ? -1 : undefined}
  onclick={onSelect}
  onpointerdown={onPointerDown}
>
  {#if unpointed}
    <span class="absolute inset-y-0 left-0 w-1 bg-warning" aria-hidden="true"
    ></span>
  {/if}
  <div class="flex items-start justify-between gap-2">
    {#if task.customId}
      <span class="font-mono text-[11px] text-muted-foreground"
        >{task.customId}</span
      >
    {:else}
      <span></span>
    {/if}
    {#if unpointed}
      <span
        class="inline-flex shrink-0 items-center gap-1 rounded-md border border-warning/50 bg-warning/15 px-1.5 py-0.5 text-[10px] font-medium text-warning"
      >
        <TriangleAlert class="size-3" aria-hidden="true" />
        No points
      </span>
    {:else}
      <span
        class="inline-flex min-w-6 shrink-0 justify-center rounded-md border border-border bg-muted/60 px-1.5 py-0.5 font-mono text-[11px] font-medium"
        title="Sprint points">{formatPoints(task.points!)}</span
      >
    {/if}
  </div>
  <p class="line-clamp-3 text-sm leading-snug font-medium">{task.name}</p>
</button>
