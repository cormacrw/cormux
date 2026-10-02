<script lang="ts">
  import { tick } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import * as Empty from '$lib/components/ui/empty'
  import * as Resizable from '$lib/components/ui/resizable'
  import {
    daysLeft,
    formatPoints,
    sprintRange,
    sprintTotals,
    statusLabel,
  } from '$lib/clickup/board'
  import type { ClickupTask } from '$lib/ipc/bindings'
  import { plural } from '$lib/workspace/plural'
  import { app, clickup, settings } from '$lib/state'
  import { cn } from '$lib/utils'
  import Columns3 from '@lucide/svelte/icons/columns-3'
  import EyeOff from '@lucide/svelte/icons/eye-off'
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert'
  import SprintTaskCard from './SprintTaskCard.svelte'
  import SprintTaskPanel from './SprintTaskPanel.svelte'

  /** Pointer travel before a press becomes a drag, so a click still selects. */
  const DRAG_THRESHOLD = 4
  const EDGE_SCROLL = 64

  let title: HTMLHeadingElement | undefined = $state()
  let boardEl: HTMLDivElement | undefined = $state()

  $effect(() => {
    if (app.focusTarget !== 'sprint') return
    void app.focusGeneration
    if (!clickup.selectedTaskId) title?.focus()
  })

  $effect(() => {
    if (app.view === 'sprint' && !clickup.board) void clickup.refresh()
  })

  const sprint = $derived(clickup.board?.sprint ?? null)
  const totals = $derived(sprintTotals(clickup.board?.tasks ?? []))
  const range = $derived(
    sprint ? sprintRange(sprint.startMs, sprint.dueMs) : null,
  )
  const remaining = $derived(sprint ? daysLeft(sprint.dueMs, Date.now()) : null)
  const meta = $derived(
    [
      range,
      remaining == null
        ? null
        : remaining === 0
          ? 'Ends today'
          : `${plural(remaining, 'day')} left`,
    ]
      .filter(Boolean)
      .join(' · '),
  )

  type Drag = {
    task: ClickupTask
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
  /** The click that ends a drag must not also select the card. */
  let swallowClick = false

  function onCardPointerDown(event: PointerEvent, task: ClickupTask) {
    if (event.button !== 0) return
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
    drag = {
      task,
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
      ? ((el.closest('[data-lane]') as HTMLElement | null)?.dataset.lane ??
          null)
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
    }
  }

  function onPointerUp() {
    if (drag?.active) {
      swallowClick = true
      setTimeout(() => (swallowClick = false), 0)
      if (overLane) void clickup.moveTask(drag.task.id, overLane)
    }
    endDrag()
  }

  function onDragKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || !drag?.active) return
    event.preventDefault()
    event.stopPropagation()
    swallowClick = true
    setTimeout(() => (swallowClick = false), 0)
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

  function selectTask(task: ClickupTask) {
    if (swallowClick) return
    clickup.select(clickup.selectedTaskId === task.id ? null : task.id)
  }

  async function closePanel() {
    const id = clickup.selectedTaskId
    clickup.select(null)
    await tick()
    if (id) {
      document.querySelector<HTMLElement>(`[data-task-id="${id}"]`)?.focus()
    }
  }

  /** Escape closes the task pane, unless a dialog, menu or select is open and wants it. */
  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented) return
    if (!clickup.selectedTaskId || drag) return
    if (
      document.querySelector(
        '[role="dialog"], [role="alertdialog"], [role="listbox"], [role="menu"]',
      )
    ) {
      return
    }
    event.preventDefault()
    void closePanel()
  }

  function openClickupSettings() {
    app.openSettings()
    settings.focusSection = 'clickup'
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<section
  class={cn(
    'view-sprint flex min-h-0 flex-1 flex-col',
    drag?.active && 'cursor-grabbing',
  )}
  aria-labelledby="sprint-title"
  data-od-id="sprint-view"
>
  <header
    class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b border-border px-6 pt-6 pb-4"
  >
    <div class="grid min-w-0 gap-1">
      <h1
        bind:this={title}
        id="sprint-title"
        tabindex="-1"
        class="text-2xl font-semibold tracking-tight outline-none"
      >
        {sprint?.name ?? 'Sprint'}
      </h1>
      {#if sprint}
        <p class="text-xs text-muted-foreground" data-od-id="sprint-meta">
          {meta}
        </p>
      {/if}
    </div>
    <div class="flex items-center gap-3">
      {#if clickup.board}
        <span class="font-mono text-xs text-muted-foreground"
          >{formatPoints(totals.points)} pts</span
        >
        {#if totals.unpointed > 0}
          <span
            class="inline-flex items-center gap-1.5 rounded-md border border-warning/50 bg-warning/15 px-2 py-1 text-xs font-medium text-warning"
            data-od-id="sprint-unpointed-count"
          >
            <TriangleAlert class="size-3.5" aria-hidden="true" />
            {plural(totals.unpointed, 'task')} without points
          </span>
        {/if}
      {/if}
      {#if clickup.board}
        <DropdownMenu.Root>
          <DropdownMenu.Trigger>
            {#snippet child({ props })}
              <Button
                {...props}
                variant="outline"
                size="sm"
                class="gap-1.5"
                data-od-id="sprint-lanes-menu"
              >
                <Columns3 class="size-3.5" aria-hidden="true" />
                Lanes
                {#if clickup.hiddenLaneCount > 0}
                  <span class="font-mono text-muted-foreground"
                    >{clickup.hiddenLaneCount} hidden</span
                  >
                {/if}
              </Button>
            {/snippet}
          </DropdownMenu.Trigger>
          <DropdownMenu.Content align="end" class="w-56">
            <DropdownMenu.Label class="text-xs text-muted-foreground">
              Show lanes
            </DropdownMenu.Label>
            {#each clickup.allLanes as lane (lane.status.name)}
              <DropdownMenu.CheckboxItem
                checked={!clickup.isHidden(lane.status.name)}
                closeOnSelect={false}
                class="data-[state=unchecked]:text-muted-foreground"
                onCheckedChange={(checked) =>
                  clickup.setLaneHidden(lane.status.name, !checked)}
              >
                <span
                  class="size-2 shrink-0 rounded-full"
                  style:background-color={lane.status.color ?? 'currentColor'}
                  aria-hidden="true"
                ></span>
                <span class="truncate">{statusLabel(lane.status.name)}</span>
                <span class="ml-auto font-mono text-xs text-muted-foreground"
                  >{lane.tasks.length}</span
                >
              </DropdownMenu.CheckboxItem>
            {/each}
            {#if clickup.hiddenLaneCount > 0}
              <DropdownMenu.Separator />
              <DropdownMenu.Item onSelect={() => clickup.showAllLanes()}>
                Show all lanes
              </DropdownMenu.Item>
            {/if}
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      {/if}
      <Button
        variant="outline"
        size="sm"
        class="gap-1.5"
        disabled={!clickup.ready || clickup.loading}
        onclick={() => void clickup.refresh()}
      >
        <RefreshCw
          class={cn('size-3.5', clickup.loading && 'animate-spin')}
          aria-hidden="true"
        />
        Refresh
      </Button>
    </div>
  </header>

  {#if !clickup.ready}
    <div class="p-6">
      <Empty.Root class="border border-dashed border-border/80">
        <Empty.Header>
          <Empty.Title>Choose your sprint folder</Empty.Title>
          <Empty.Description>
            Pick the ClickUp workspace, space and sprint folder in Settings. The
            board shows the sprint whose dates include today.
          </Empty.Description>
        </Empty.Header>
        <Empty.Content>
          <Button variant="secondary" onclick={openClickupSettings}>
            Open ClickUp settings
          </Button>
        </Empty.Content>
      </Empty.Root>
    </div>
  {:else if !clickup.board}
    <div class="p-6">
      {#if clickup.error}
        <Empty.Root class="border border-dashed border-border/80">
          <Empty.Header>
            <Empty.Title>Couldn't load the sprint</Empty.Title>
            <Empty.Description>{clickup.error}</Empty.Description>
          </Empty.Header>
          <Empty.Content>
            <Button variant="secondary" onclick={() => void clickup.refresh()}>
              Try again
            </Button>
          </Empty.Content>
        </Empty.Root>
      {:else}
        <p class="text-sm text-muted-foreground" role="status">
          Loading the sprint…
        </p>
      {/if}
    </div>
  {:else}
    {#if clickup.error}
      <p
        class="border-b border-border bg-destructive/10 px-6 py-2 text-xs text-destructive"
        role="status"
      >
        Showing the last loaded board. {clickup.error}
      </p>
    {/if}
    <Resizable.PaneGroup
      direction="horizontal"
      autoSaveId="cormux-sprint-board"
      class="min-h-0 flex-1"
    >
      <Resizable.Pane id="sprint-board" order={1} minSize={35}>
        <div
          bind:this={boardEl}
          class="flex h-full min-h-0 items-start gap-3 overflow-x-auto overflow-y-hidden p-4"
          role="list"
          aria-label="Sprint lanes"
          data-od-id="sprint-board"
        >
          {#each clickup.lanes as lane (lane.status.name)}
            {@const isTarget =
              drag?.active &&
              overLane === lane.status.name &&
              drag.task.status.toLowerCase() !== lane.status.name.toLowerCase()}
            <section
              class={cn(
                'group/lane flex max-h-full w-[272px] shrink-0 flex-col rounded-xl border border-transparent bg-muted/40 transition-colors',
                isTarget && 'border-primary/60 bg-primary/[0.07]',
              )}
              role="listitem"
              aria-label="{statusLabel(lane.status.name)}, {plural(
                lane.tasks.length,
                'task',
              )}"
              data-lane={lane.status.name}
            >
              <header class="flex items-center gap-2 px-3 pt-3 pb-2">
                <span
                  class="size-2.5 shrink-0 rounded-full"
                  style:background-color={lane.status.color ?? 'currentColor'}
                  aria-hidden="true"
                ></span>
                <h2
                  class="truncate text-xs font-semibold tracking-wide uppercase"
                >
                  {statusLabel(lane.status.name)}
                </h2>
                <span class="font-mono text-xs text-muted-foreground"
                  >{lane.tasks.length}</span
                >
                {#if lane.tasks.length > 0}
                  <span
                    class="ml-auto font-mono text-[11px] text-muted-foreground"
                    title="Sprint points in this lane"
                    >{formatPoints(sprintTotals(lane.tasks).points)} pts</span
                  >
                {/if}
                <button
                  type="button"
                  class={cn(
                    'flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground opacity-0 outline-none transition-opacity group-hover/lane:opacity-100 hover:bg-muted hover:text-foreground focus-visible:opacity-100 focus-visible:ring-3 focus-visible:ring-ring/50',
                    lane.tasks.length === 0 && 'ml-auto',
                  )}
                  aria-label="Hide {statusLabel(lane.status.name)}"
                  title="Hide lane"
                  onclick={() => clickup.setLaneHidden(lane.status.name, true)}
                >
                  <EyeOff class="size-3.5" aria-hidden="true" />
                </button>
              </header>
              <div
                class="flex min-h-16 flex-col gap-2 overflow-y-auto px-2 pt-1 pb-2"
              >
                {#each lane.tasks as task (task.id)}
                  <SprintTaskCard
                    {task}
                    selected={clickup.selectedTaskId === task.id}
                    dragging={drag?.active === true && drag.task.id === task.id}
                    onSelect={() => selectTask(task)}
                    onPointerDown={(event) => onCardPointerDown(event, task)}
                  />
                {/each}
              </div>
            </section>
          {:else}
            {#if clickup.hiddenLaneCount > 0}
              <p
                class="flex items-center gap-2 p-2 text-sm text-muted-foreground"
              >
                Every lane is hidden.
                <Button
                  variant="link"
                  size="sm"
                  class="h-auto p-0"
                  onclick={() => clickup.showAllLanes()}
                >
                  Show all lanes
                </Button>
              </p>
            {:else}
              <p class="p-2 text-sm text-muted-foreground">
                This sprint has no tasks yet.
              </p>
            {/if}
          {/each}
        </div>
      </Resizable.Pane>
      {#if clickup.selectedTask}
        <Resizable.Handle withHandle />
        <Resizable.Pane
          id="sprint-task"
          order={2}
          defaultSize={34}
          minSize={22}
          maxSize={60}
        >
          <SprintTaskPanel />
        </Resizable.Pane>
      {/if}
    </Resizable.PaneGroup>
  {/if}
</section>

{#if drag?.active}
  <div
    class="pointer-events-none fixed z-50"
    style:left="{drag.x - drag.offsetX}px"
    style:top="{drag.y - drag.offsetY}px"
    style:width="{drag.width}px"
    aria-hidden="true"
  >
    <SprintTaskCard task={drag.task} ghost />
  </div>
{/if}
