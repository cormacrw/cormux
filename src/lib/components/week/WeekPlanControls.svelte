<script lang="ts">
  import { dayLabel, type Day } from '$lib/week/dates'
  import type { WeekItem } from '$lib/week/items'
  import { cn } from '$lib/utils'
  import Check from '@lucide/svelte/icons/check'

  let {
    item,
    days,
    onPlan,
    onToggleDone,
  }: {
    item: WeekItem
    /** The days on screen, Monday first. */
    days: Day[]
    onPlan: (day: Day | null) => void
    onToggleDone: () => void
  } = $props()

  const options = $derived([
    { day: null, label: 'Unplanned', long: 'Unplanned' },
    ...days.map((day) => {
      const label = dayLabel(day)
      return { day, label: label.name, long: `${label.long}, ${label.date}` }
    }),
  ])
  const elsewhere = $derived(
    item.plannedOn && !days.includes(item.plannedOn)
      ? (() => {
          const label = dayLabel(item.plannedOn)
          return `${label.name}, ${label.date}`
        })()
      : null,
  )
  const labelId = $derived(`week-plan-label-${item.key}`)

  let group: HTMLDivElement | undefined = $state()

  /** Arrow keys move the choice, as in any radio group. */
  function onKeydown(event: KeyboardEvent) {
    const step =
      event.key === 'ArrowRight' || event.key === 'ArrowDown'
        ? 1
        : event.key === 'ArrowLeft' || event.key === 'ArrowUp'
          ? -1
          : 0
    if (!step) return
    event.preventDefault()
    const current = options.findIndex((option) => option.day === item.plannedOn)
    const next =
      options[(Math.max(current, 0) + step + options.length) % options.length]!
    onPlan(next.day)
    queueMicrotask(() =>
      group
        ?.querySelector<HTMLElement>(`[data-plan-day="${next.day ?? ''}"]`)
        ?.focus(),
    )
  }
</script>

<div class="grid gap-4 border-t border-border pt-4">
  <div class="grid gap-2">
    <span id={labelId} class="text-xs font-semibold">Planned for</span>
    <div
      bind:this={group}
      class="flex flex-wrap gap-1"
      role="radiogroup"
      aria-labelledby={labelId}
      tabindex={-1}
      onkeydown={onKeydown}
    >
      {#each options as option (option.day ?? 'none')}
        {@const checked = item.plannedOn === option.day}
        <button
          type="button"
          role="radio"
          aria-checked={checked}
          aria-label={option.long}
          tabindex={checked || (option.day === null && elsewhere) ? 0 : -1}
          data-plan-day={option.day ?? ''}
          class={cn(
            'h-7 rounded-lg border px-2.5 text-xs outline-none transition-[transform,opacity] duration-150 focus-visible:ring-3 focus-visible:ring-ring/50 active:scale-95',
            checked
              ? 'border-primary bg-primary text-primary-foreground'
              : 'glass-well border-border hover:bg-muted',
          )}
          onclick={() => onPlan(option.day)}>{option.label}</button
        >
      {/each}
    </div>
    {#if elsewhere}
      <p class="text-xs text-muted-foreground">Planned for {elsewhere}.</p>
    {/if}
  </div>

  <button
    type="button"
    class={cn(
      'inline-flex h-9 items-center justify-center gap-2 rounded-[10px] border text-[13px] font-semibold outline-none transition-[transform,opacity] duration-150 focus-visible:ring-3 focus-visible:ring-ring/50 active:scale-[0.98] disabled:opacity-60',
      item.done
        ? 'border-success/50 bg-success/15 text-success'
        : 'border-border bg-muted/60 hover:bg-muted',
    )}
    aria-pressed={item.done}
    disabled={item.doneInClickup}
    title={item.doneInClickup ? 'Done in ClickUp' : undefined}
    data-od-id="week-toggle-done"
    onclick={onToggleDone}
  >
    <Check class="size-3.5" strokeWidth={2.6} aria-hidden="true" />
    {item.doneInClickup
      ? 'Done in ClickUp'
      : item.done
        ? 'Completed · mark not done'
        : 'Mark complete'}
  </button>
</div>
