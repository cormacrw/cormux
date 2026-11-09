<!--
  The task flyout: a sprint task opened on the Sprint page slides out from the right over the page, like Git and Output in a workspace.
  Drag its left edge to resize it; Esc or × closes it.
-->
<script lang="ts">
  import SprintTaskPanel from '$lib/clickup/components/SprintTaskPanel.svelte'
  import { clickup } from '$lib/clickup/store.svelte'
  import { overlaySelector } from '$lib/keyboard/focus-home'
  import {
    readPaneWidth,
    resizeEdge,
    savePaneWidth,
  } from '$lib/primitives/slide-out'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'

  let {
    focusTask,
    isTaskCard,
  }: {
    /** Back to the task's card on the board. */
    focusTask: (taskId: string) => void
    /** Whether a click landed on a task card, which opens that task rather than closing this. */
    isTaskCard: (target: Element) => boolean
  } = $props()

  const WIDTH_KEY = 'cormux-task-flyout-width'

  let width = $state(readPaneWidth(WIDTH_KEY, 340, 440))

  const ready = $derived(clickup.selectedTask != null)

  /** Esc, × and a click elsewhere: focus goes back to the task's card, since the pane turns inert. */
  function close() {
    const id = clickup.selectedTaskId
    clickup.select(null)
    if (!id) return
    focusTask(id)
  }

  $effect(() =>
    keyboardLayers.onEscape('slideOut', () => {
      if (!ready) return false
      close()
      return true
    }),
  )

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
      isTaskCard(target) ||
      target.closest(
        `${overlaySelector()}, [data-bits-floating-content-wrapper]`,
      )
    )
      return
    close()
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} />

<div
  bind:this={flyoutEl}
  class="slide-out glass-pane absolute inset-y-3 right-3 z-40 flex max-w-[calc(100%-24px)] flex-col overflow-hidden rounded-[18px]"
  style:width="{width}px"
  data-open={ready}
  inert={!ready}
  data-od-id="task-flyout"
>
  <div
    class="absolute inset-y-0 left-0 z-10 w-1.5 cursor-col-resize"
    title="Drag to resize"
    data-od-id="task-flyout-resize"
    use:resizeEdge={{
      bounds: () => flyoutEl?.offsetParent?.getBoundingClientRect(),
      min: 340,
      reserve: 332,
      onResize: (next) => (width = next),
      onEnd: () => savePaneWidth(WIDTH_KEY, width),
    }}
  ></div>
  <!-- Stays mounted while it slides away; the pane holds its own last task. -->
  <div class="min-h-0 flex-1">
    <SprintTaskPanel onClose={close} />
  </div>
</div>
