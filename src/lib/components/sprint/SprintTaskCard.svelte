<script lang="ts">
  import type { ClickupTask } from '$lib/ipc/bindings'
  import { formatPoints, statusLabel, taskRef } from '$lib/clickup/board'
  import { cn } from '$lib/utils'
  import CircleDashed from '@lucide/svelte/icons/circle-dashed'
  import Flag from '@lucide/svelte/icons/flag'
  import TaskActions from './TaskActions.svelte'

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
  const ref = $derived(taskRef(task))

  function pointerDown(event: PointerEvent) {
    if ((event.target as Element).closest('[data-card-action]')) return
    onPointerDown?.(event)
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class={cn(
    'group/card relative flex touch-none flex-col gap-3 rounded-xl border bg-card p-3 text-card-foreground select-none',
    'shadow-[0_1px_2px_rgb(0_0_0/0.18)] transition-[border-color,background-color,box-shadow,opacity,transform] duration-150',
    'has-[[data-task-id]:focus-visible]:ring-3 has-[[data-task-id]:focus-visible]:ring-ring/50',
    unpointed
      ? 'border-dashed border-warning/70 bg-warning/[0.04]'
      : 'border-border/70 [@media(hover:hover)_and_(pointer:fine)]:hover:border-foreground/25',
    !ghost && '[@media(hover:hover)_and_(pointer:fine)]:hover:-translate-y-px',
    selected && 'border-primary/80 ring-2 ring-primary/25',
    dragging && 'opacity-35',
    ghost && 'rotate-[1.5deg] cursor-grabbing shadow-2xl',
  )}
  data-unpointed={unpointed || undefined}
  onpointerdown={ghost ? undefined : pointerDown}
>
  {#if !ghost}
    <!-- Covers the card so the whole surface selects, while the action buttons sit above it. -->
    <button
      type="button"
      class="absolute inset-0 rounded-[inherit] outline-none"
      aria-pressed={selected}
      aria-label="{ref} {task.name}, {statusLabel(task.status)}, {unpointed
        ? 'no sprint points'
        : `${formatPoints(task.points!)} points`}"
      data-task-id={task.id}
      onclick={onSelect}
    ></button>
    <TaskActions
      {task}
      class="absolute top-2 right-2 z-10 rounded-lg border border-border/70 bg-card/95 p-0.5 opacity-0 shadow-sm transition-opacity group-focus-within/card:opacity-100 [@media(hover:hover)_and_(pointer:fine)]:group-hover/card:opacity-100"
    />
  {/if}

  <p
    class="pointer-events-none line-clamp-3 text-[13px] leading-snug font-medium"
  >
    {task.name}
  </p>

  <div class="pointer-events-none flex min-w-0 items-center gap-2">
    <span class="truncate font-mono text-[11px] text-muted-foreground"
      >{ref}</span
    >
    {#if task.priority}
      <Flag
        class="size-3 shrink-0"
        style="color: {task.priorityColor ?? 'currentColor'}"
        aria-label="{statusLabel(task.priority)} priority"
      />
    {/if}

    <div class="ml-auto shrink-0">
      {#if unpointed}
        <span
          class="inline-flex items-center gap-1 rounded-full border border-dashed border-warning/80 bg-warning/15 px-1.5 py-px text-[10px] font-semibold text-warning"
        >
          <CircleDashed class="size-3" aria-hidden="true" />
          No pts
        </span>
      {:else}
        <span
          class="inline-flex items-baseline gap-0.5 rounded-full bg-muted px-2 py-px font-mono text-[11px] font-semibold"
          title="Sprint points"
          >{formatPoints(task.points!)}<span
            class="text-[9px] font-normal text-muted-foreground">pts</span
          ></span
        >
      {/if}
    </div>
  </div>
</div>
