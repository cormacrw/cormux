<script lang="ts">
  import { untrack } from 'svelte'
  import { planItem, toggleItemDone } from '$lib/week/actions'
  import { fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import { Button } from '$lib/components/ui/button'
  import { motionMs } from '$lib/motion'
  import { app, clickup, todos, weekPlan } from '$lib/state'
  import { cn } from '$lib/utils'
  import { addDays, dayLabel, weekRange } from '$lib/week/dates'
  import {
    byDay,
    itemKey,
    loadingItems,
    plannedDone,
    weekItems,
    type WeekFilter,
    type WeekItem,
  } from '$lib/week/items'
  import ChevronLeft from '@lucide/svelte/icons/chevron-left'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import Inbox from '@lucide/svelte/icons/inbox'
  import WeekCard from './WeekCard.svelte'
  import { Switch } from '$lib/components/ui/switch'

  /** Pointer travel before a press becomes a drag, so a click still opens the card. */
  const DRAG_THRESHOLD = 4
  const EDGE_SCROLL = 64
  const LOADING = 'loading'

  let title: HTMLHeadingElement | undefined = $state()
  let boardEl: HTMLDivElement | undefined = $state()
  let draft = $state('')

  $effect(() => {
    if (app.focusTarget !== 'week') return
    void app.focusGeneration
    untrack(() => {
      weekPlan.refreshToday()
      if (!weekPlan.selected()) title?.focus()
    })
  })

  $effect(() => {
    if (app.view === 'week' && clickup.ready && !clickup.board) {
      void clickup.refresh()
    }
  })

  const items = $derived(
    weekItems(
      todos.items,
      clickup.ready ? (clickup.mine ?? null) : null,
      weekPlan.rows,
    ),
  )
  const loading = $derived.by(() => {
    const waiting = loadingItems(items, {
      today: weekPlan.today,
      thisWeek: weekPlan.thisWeek,
      shown: weekPlan.days,
      filter: weekPlan.filter,
    })
    // With weekends hidden, what's planned on them waits here instead of vanishing.
    const hidden = weekPlan.days.filter(
      (day) => !weekPlan.visibleDays.includes(day),
    )
    const onHiddenDays = items.filter(
      (item) =>
        !item.done &&
        item.plannedOn != null &&
        hidden.includes(item.plannedOn) &&
        (weekPlan.filter === 'all' || item.kind === weekPlan.filter),
    )
    return [...waiting, ...onHiddenDays]
  })
  const lanes = $derived(byDay(items, weekPlan.visibleDays))
  const progress = $derived(plannedDone(items, weekPlan.visibleDays))
  const heading = $derived.by(() => {
    if (weekPlan.isThisWeek) return 'This week'
    if (weekPlan.start === addDays(weekPlan.thisWeek, -7)) return 'Last week'
    if (weekPlan.start === addDays(weekPlan.thisWeek, 7)) return 'Next week'
    return `Week of ${dayLabel(weekPlan.start).date}`
  })

  const selection = $derived(weekPlan.selected())
  const selectedItem = $derived(
    selection
      ? (items.find(
          (item) => item.key === itemKey(selection.kind, selection.id),
        ) ?? null)
      : null,
  )
  const FILTERS: { id: WeekFilter; label: string }[] = [
    { id: 'all', label: 'All' },
    { id: 'todo', label: 'TODOs' },
    { id: 'sprint', label: 'Sprint' },
  ]

  async function add() {
    const value = draft.trim()
    if (!value) return
    // Clear first so the next one can be typed while this one saves.
    draft = ''
    if (!(await todos.add(value)) && !draft) draft = value
  }

  function onAddKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.isComposing) {
      event.preventDefault()
      void add()
    } else if (event.key === 'Escape' && draft) {
      event.preventDefault()
      event.stopPropagation()
      draft = ''
    }
  }

  // The flyout (TaskFlyout, at the app level) shares these.
  const plan = planItem
  const toggleDone = toggleItemDone

  type Drag = {
    item: WeekItem
    from: string
    startX: number
    startY: number
    x: number
    y: number
    offsetX: number
    offsetY: number
    width: number
    active: boolean
  }
  let drag = $state<Drag | null>(null)
  let overLane = $state<string | null>(null)
  /** The click that ends a drag must not also open the card. */
  let swallowClick = false

  function laneOf(item: WeekItem) {
    return item.plannedOn && weekPlan.visibleDays.includes(item.plannedOn)
      ? item.plannedOn
      : LOADING
  }

  function onCardPointerDown(event: PointerEvent, item: WeekItem) {
    if (event.button !== 0) return
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
    drag = {
      item,
      from: laneOf(item),
      startX: event.clientX,
      startY: event.clientY,
      x: event.clientX,
      y: event.clientY,
      offsetX: event.clientX - rect.left,
      offsetY: event.clientY - rect.top,
      width: rect.width,
      active: false,
    }
    window.addEventListener('pointermove', onPointerMove)
    window.addEventListener('pointerup', onPointerUp)
    window.addEventListener('pointercancel', endDrag)
    window.addEventListener('keydown', onDragKeydown)
  }

  function laneAt(x: number, y: number) {
    const el = document.elementFromPoint(x, y)
    return el instanceof Element
      ? ((el.closest('[data-week-lane]') as HTMLElement | null)?.dataset
          .weekLane ?? null)
      : null
  }

  function onPointerMove(event: PointerEvent) {
    if (!drag) return
    drag.x = event.clientX
    drag.y = event.clientY
    if (
      !drag.active &&
      Math.hypot(drag.x - drag.startX, drag.y - drag.startY) < DRAG_THRESHOLD
    ) {
      return
    }
    drag.active = true
    overLane = laneAt(drag.x, drag.y)
    if (boardEl) {
      const rect = boardEl.getBoundingClientRect()
      if (drag.x < rect.left + EDGE_SCROLL) boardEl.scrollLeft -= 14
      else if (drag.x > rect.right - EDGE_SCROLL) boardEl.scrollLeft += 14
      if (drag.y < rect.top + EDGE_SCROLL) boardEl.scrollTop -= 14
      else if (drag.y > rect.bottom - EDGE_SCROLL) boardEl.scrollTop += 14
    }
  }

  function swallowNextClick() {
    swallowClick = true
    setTimeout(() => (swallowClick = false), 0)
  }

  function onPointerUp(event: PointerEvent) {
    if (drag?.active) {
      swallowNextClick()
      // The lane under the pointer now: edge scrolling may have moved the lanes since the last move.
      overLane = laneAt(event.clientX, event.clientY) ?? overLane
      if (overLane && overLane !== drag.from) {
        plan(drag.item, overLane === LOADING ? null : overLane)
      }
    }
    endDrag()
  }

  function onDragKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || !drag?.active) return
    event.preventDefault()
    event.stopPropagation()
    swallowNextClick()
    endDrag()
  }

  function endDrag() {
    drag = null
    overLane = null
    window.removeEventListener('pointermove', onPointerMove)
    window.removeEventListener('pointerup', onPointerUp)
    window.removeEventListener('pointercancel', endDrag)
    window.removeEventListener('keydown', onDragKeydown)
  }

  $effect(() => endDrag)

  function open(item: WeekItem) {
    if (swallowClick) return
    const same = selectedItem?.key === item.key
    weekPlan.select(same ? null : { kind: item.kind, id: item.id })
  }
</script>

{#snippet card(item: WeekItem)}
  <div in:fly={{ y: 6, duration: motionMs(160), easing: cubicOut }}>
    <WeekCard
      {item}
      selected={selectedItem?.key === item.key}
      dragging={drag?.active === true && drag.item.key === item.key}
      onOpen={() => open(item)}
      onToggleDone={() => toggleDone(item)}
      onPointerDown={(event) => onCardPointerDown(event, item)}
    />
  </div>
{/snippet}

{#snippet dropHighlight(key: string)}
  <span
    class={cn(
      'pointer-events-none absolute inset-0 rounded-[inherit] border border-primary/60 bg-primary/10 opacity-0 transition-opacity duration-150 motion-reduce:transition-none',
      drag?.active && overLane === key && drag.from !== key && 'opacity-100',
    )}
    aria-hidden="true"
  ></span>
{/snippet}

<section
  class={cn(
    'view-week flex min-h-0 flex-1 flex-col',
    drag?.active && 'cursor-grabbing',
  )}
  aria-labelledby="week-title"
  data-od-id="week-view"
>
  <header class="flex flex-wrap items-center gap-x-4 gap-y-2 px-6 pt-6 pb-4">
    <div class="grid min-w-0 gap-0.5">
      <h1
        bind:this={title}
        id="week-title"
        tabindex="-1"
        class="text-2xl font-semibold tracking-tight outline-none"
      >
        {heading}
      </h1>
      <p class="text-[13px] text-muted-foreground" data-od-id="week-meta">
        {weekRange(weekPlan.start)} ·
        <span data-od-id="week-progress"
          >{progress.done} of {progress.total} planned done</span
        >
      </p>
    </div>
    <label
      class="ml-auto flex items-center gap-2 text-xs text-muted-foreground"
      data-od-id="week-weekends"
    >
      <Switch
        checked={!weekPlan.hideWeekends}
        onCheckedChange={(show) => weekPlan.setHideWeekends(!show)}
        aria-label="Show weekends"
      />
      Weekends
    </label>
    <div class="flex items-center gap-0.5" role="group" aria-label="Week">
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label="Previous week"
        onclick={() => weekPlan.previousWeek()}
      >
        <ChevronLeft class="size-4" aria-hidden="true" />
      </Button>
      <Button
        variant="ghost"
        size="sm"
        aria-pressed={weekPlan.isThisWeek}
        onclick={() => weekPlan.goToday()}>Today</Button
      >
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label="Next week"
        onclick={() => weekPlan.nextWeek()}
      >
        <ChevronRight class="size-4" aria-hidden="true" />
      </Button>
    </div>
  </header>

  <div class="min-h-0 flex-1">
    <div
      bind:this={boardEl}
      class="flex h-full min-h-0 items-stretch gap-2.5 overflow-auto px-6 pt-1 pb-6"
      data-od-id="week-board"
    >
      <section
        class="glass-card relative flex min-h-full w-[270px] shrink-0 flex-col gap-2 self-start rounded-2xl p-2.5"
        aria-labelledby="week-loading-title"
        data-week-lane={LOADING}
        data-od-id="week-loading"
      >
        {@render dropHighlight(LOADING)}
        <div class="flex items-center gap-2 px-1 pt-0.5">
          <Inbox class="size-3.5 opacity-80" aria-hidden="true" />
          <h2 id="week-loading-title" class="text-[13px] font-bold">
            Loading area
          </h2>
          <span class="font-mono text-[11px] text-muted-foreground"
            >{loading.length}</span
          >
        </div>
        <p class="px-1 text-[11px] leading-snug text-muted-foreground">
          Open TODOs{clickup.ready ? ' and your sprint tasks' : ''}. Drag them
          onto a day.
        </p>
        <label for="week-add" class="sr-only">Add a TODO</label>
        <input
          id="week-add"
          class="glass-well relative h-[34px] w-full rounded-[10px] px-3 text-[13px] outline-none placeholder:text-muted-foreground/70 focus-visible:ring-3 focus-visible:ring-ring/50"
          type="text"
          autocomplete="off"
          placeholder="+ Add a TODO"
          bind:value={draft}
          onkeydown={onAddKeydown}
        />
        <div
          class="glass-well relative inline-flex self-start rounded-lg p-0.5"
          role="group"
          aria-label="Show"
        >
          {#each FILTERS as filter (filter.id)}
            <button
              type="button"
              class={cn(
                'h-6 rounded-md px-2.5 text-xs outline-none transition-opacity focus-visible:ring-3 focus-visible:ring-ring/50',
                weekPlan.filter === filter.id
                  ? 'bg-muted text-foreground'
                  : 'text-muted-foreground hover:text-foreground',
              )}
              aria-pressed={weekPlan.filter === filter.id}
              onclick={() => (weekPlan.filter = filter.id)}
              >{filter.label}</button
            >
          {/each}
        </div>
        <div class="relative flex flex-col gap-2" role="list">
          {#each loading as item (item.key)}
            <div role="listitem">{@render card(item)}</div>
          {:else}
            <p class="px-1 py-3 text-xs text-muted-foreground">
              Nothing waiting to be planned.
            </p>
          {/each}
        </div>
      </section>

      {#each weekPlan.visibleDays as day (day)}
        {@const label = dayLabel(day)}
        {@const cards = lanes.get(day) ?? []}
        {@const today = day === weekPlan.today}
        <!-- Styled like the Sprint board's lanes: as tall as their cards, scrolling past the board. -->
        <section
          class={cn(
            'group/lane relative flex max-h-full w-[300px] shrink-0 flex-col self-start rounded-xl border border-transparent bg-muted/40 transition-colors',
            today && 'border-info/30',
          )}
          aria-label="{label.long} {label.date}{today ? ', today' : ''}"
          data-week-lane={day}
          data-today={today || undefined}
        >
          {@render dropHighlight(day)}
          <header class="flex items-center gap-2 px-3 pt-3 pb-2">
            <span
              class={cn(
                'size-2.5 shrink-0 rounded-full',
                today ? 'bg-info' : 'bg-muted-foreground/40',
              )}
              aria-hidden="true"
            ></span>
            <h2
              class={cn(
                'truncate text-xs font-semibold tracking-wide uppercase',
                today && 'text-info',
              )}
            >
              {label.name}
            </h2>
            <span class="text-xs text-muted-foreground">{label.date}</span>
            <span class="font-mono text-xs text-muted-foreground"
              >{cards.length}</span
            >
            {#if today}
              <span
                class="ml-auto rounded-full bg-info/25 px-1.5 py-px text-[10px] font-bold text-info"
                >Today</span
              >
            {/if}
          </header>
          <div
            class="relative flex min-h-16 flex-1 flex-col gap-2 overflow-y-auto px-2 pt-1 pb-2"
            role="list"
          >
            {#each cards as item (item.key)}
              <div role="listitem">{@render card(item)}</div>
            {:else}
              <div
                class="grid h-16 place-items-center rounded-lg border border-dashed border-border text-[11px] text-muted-foreground"
              >
                Drop tasks here
              </div>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  </div>
</section>

{#if drag?.active}
  <div
    class="pointer-events-none fixed z-50"
    style:left="{drag.x - drag.offsetX}px"
    style:top="{drag.y - drag.offsetY}px"
    style:width="{drag.width}px"
    aria-hidden="true"
  >
    <WeekCard item={drag.item} ghost />
  </div>
{/if}
