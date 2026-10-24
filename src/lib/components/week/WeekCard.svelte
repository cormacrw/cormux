<script lang="ts">
  import { formatPoints } from '$lib/clickup/board'
  import type { WeekItem } from '$lib/week/items'
  import { cn } from '$lib/utils'
  import Check from '@lucide/svelte/icons/check'

  let {
    item,
    selected = false,
    dragging = false,
    ghost = false,
    onOpen,
    onToggleDone,
    onPointerDown,
  }: {
    item: WeekItem
    selected?: boolean
    /** The card is being dragged; this copy stays behind as a placeholder. */
    dragging?: boolean
    /** The copy that follows the pointer. */
    ghost?: boolean
    onOpen?: () => void
    onToggleDone?: () => void
    onPointerDown?: (event: PointerEvent) => void
  } = $props()

  const checkLabel = $derived(
    item.doneInClickup
      ? `“${item.title}” is done in ClickUp`
      : item.done
        ? `Mark “${item.title}” not done`
        : `Mark “${item.title}” done`,
  )

  function pointerDown(event: PointerEvent) {
    if ((event.target as Element).closest('[data-card-action]')) return
    onPointerDown?.(event)
  }

  function toggle(event: MouseEvent) {
    event.stopPropagation()
    onToggleDone?.()
  }
</script>

<article
  class={cn(
    'glass-card group/card relative flex touch-none items-start gap-2 rounded-xl py-2.5 pr-2.5 pl-2 text-card-foreground select-none',
    'transition-[transform,opacity] duration-150 ease-out motion-reduce:transition-none',
    'has-[[data-week-card]:focus-visible]:ring-3 has-[[data-week-card]:focus-visible]:ring-ring/50',
    !ghost &&
      'cursor-grab [@media(hover:hover)_and_(pointer:fine)]:hover:-translate-y-px',
    item.done && !ghost && 'opacity-60',
    selected && 'border-primary/70 ring-2 ring-primary/25',
    dragging && 'opacity-35',
    ghost && 'rotate-[1.5deg] cursor-grabbing shadow-2xl',
  )}
  data-week-item={item.key}
  data-done={item.done || undefined}
  onpointerdown={ghost ? undefined : pointerDown}
>
  <button
    type="button"
    class={cn(
      'mt-px grid size-[22px] shrink-0 place-items-center rounded-full border-[1.5px] outline-none transition-[transform,opacity] duration-150 focus-visible:ring-3 focus-visible:ring-ring/50 active:scale-90 disabled:cursor-default',
      item.done
        ? 'border-success bg-success text-background'
        : 'border-muted-foreground/50 text-transparent [@media(hover:hover)_and_(pointer:fine)]:hover:border-success',
    )}
    aria-pressed={item.done}
    aria-label={checkLabel}
    title={item.doneInClickup ? 'Done in ClickUp' : undefined}
    disabled={ghost || item.doneInClickup}
    tabindex={ghost ? -1 : undefined}
    data-card-action
    onclick={toggle}
  >
    <Check class="size-3" strokeWidth={3} aria-hidden="true" />
  </button>

  <button
    type="button"
    class="flex min-w-0 flex-1 flex-col gap-1.5 text-left outline-none"
    aria-pressed={selected}
    aria-label="{item.tag} {item.title}{item.done ? ', done' : ''}"
    tabindex={ghost ? -1 : undefined}
    data-week-card={ghost ? undefined : item.key}
    onclick={onOpen}
  >
    <span
      class={cn(
        'line-clamp-3 text-[13px] leading-snug font-medium',
        item.done && 'text-muted-foreground line-through',
      )}>{item.title}</span
    >
    <span class="flex min-w-0 items-center gap-1.5">
      <span
        class={cn(
          'truncate rounded-[5px] px-1.5 py-px font-mono text-[10px] font-medium',
          item.kind === 'sprint'
            ? 'bg-info/20 text-info'
            : 'bg-accent text-accent-foreground',
        )}>{item.tag}</span
      >
      {#if item.kind === 'sprint' && item.points != null}
        <span class="shrink-0 font-mono text-[10px] text-muted-foreground"
          >{formatPoints(item.points)} pts</span
        >
      {/if}
    </span>
  </button>
</article>
