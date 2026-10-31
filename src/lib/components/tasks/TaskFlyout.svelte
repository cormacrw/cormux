<!--
  The task flyout: a sprint task opened on the Sprint page slides out from the right over the page, like Git and Output in a workspace.
  Drag its left edge to resize it; Esc or × closes it.
-->
<script lang="ts">
  import { tick, untrack } from 'svelte'
  import SprintTaskPanel from '$lib/components/sprint/SprintTaskPanel.svelte'
  import { app, clickup } from '$lib/state'

  const WIDTH_KEY = 'cormux-task-flyout-width'
  const MIN_WIDTH = 340

  function readWidth() {
    try {
      const stored = Number(localStorage.getItem(WIDTH_KEY))
      return stored >= MIN_WIDTH ? stored : 440
    } catch {
      return 440
    }
  }

  let width = $state(readWidth())

  const ready = $derived(clickup.selectedTask != null)

  // Leaving the page it was opened on closes it. A task opened along with the
  // page (app.openSprint(taskId)) belongs to that page, so it stays.
  let openedOn: string | null = null
  $effect(() => {
    void clickup.selectedTaskId
    untrack(() => (openedOn = app.view))
  })
  $effect(() => {
    const view = app.view
    untrack(() => {
      if (clickup.selectedTaskId && openedOn !== view) clickup.select(null)
    })
  })

  async function close() {
    const id = clickup.selectedTaskId
    clickup.select(null)
    await tick()
    if (!id) return
    document.querySelector<HTMLElement>(`[data-task-id="${id}"]`)?.focus()
  }

  /** Esc closes it, unless a dialog, menu or select is open and wants the key. */
  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented || !ready) return
    if (
      document.querySelector(
        '[role="dialog"], [role="alertdialog"], [role="listbox"], [role="menu"]',
      )
    ) {
      return
    }
    event.preventDefault()
    void close()
  }

  let flyoutEl: HTMLDivElement | undefined = $state()

  /**
   * A click anywhere else in the app closes it: not on a task card (that opens
   * that task instead), and not in a menu, dialog or open select.
   */
  function onWindowPointerDown(event: PointerEvent) {
    if (!ready || event.button !== 0) return
    const target = event.target
    if (!(target instanceof Element) || flyoutEl?.contains(target)) return
    if (
      target.closest(
        '[data-task-id], [role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"], [data-bits-floating-content-wrapper]',
      )
    )
      return
    void close()
  }

  // Dragging the left edge resizes it, leaving the page at least 320px.
  function onEdgeDown(event: PointerEvent) {
    if (event.button !== 0) return
    event.preventDefault()
    const handle = event.currentTarget as HTMLElement
    handle.setPointerCapture(event.pointerId)
    const area = handle.closest('.main-area')?.getBoundingClientRect()
    const move = (next: PointerEvent) => {
      if (!area) return
      width = Math.round(
        Math.min(
          Math.max(area.right - 12 - next.clientX, MIN_WIDTH),
          Math.max(MIN_WIDTH, area.width - 332),
        ),
      )
    }
    const up = () => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', up)
      handle.removeEventListener('pointercancel', up)
      try {
        localStorage.setItem(WIDTH_KEY, String(width))
      } catch {
        // Optional; the width holds for this session.
      }
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', up)
    handle.addEventListener('pointercancel', up)
  }
</script>

<svelte:window
  onkeydown={onWindowKeydown}
  onpointerdown={onWindowPointerDown}
/>

<div
  bind:this={flyoutEl}
  class="slide-out glass-pane absolute inset-y-3 right-3 z-40 flex max-w-[calc(100%-24px)] flex-col overflow-hidden rounded-[18px]"
  style:width="{width}px"
  data-open={ready}
  inert={!ready}
  data-od-id="task-flyout"
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="absolute inset-y-0 left-0 z-10 w-1.5 cursor-col-resize"
    title="Drag to resize"
    data-od-id="task-flyout-resize"
    onpointerdown={onEdgeDown}
  ></div>
  <!-- Stays mounted while it slides away; the pane holds its own last task. -->
  <div class="min-h-0 flex-1 [&>aside]:bg-transparent">
    <SprintTaskPanel />
  </div>
</div>
