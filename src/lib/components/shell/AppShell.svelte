<script lang="ts">
  import type { Snippet } from 'svelte'
  import {
    clampSidebarWidth,
    readSidebarWidth,
    SIDEBAR_MAX_WIDTH,
    SIDEBAR_MIN_WIDTH,
    writeSidebarWidth,
  } from '$lib/sidebar/width'
  import Sidebar from './Sidebar.svelte'

  let { children }: { children: Snippet } = $props()

  const KEY_STEP = 16

  // Null keeps the CSS default, which narrows on small windows.
  let width = $state<number | null>(readSidebarWidth())
  let aside: HTMLElement | undefined = $state()
  let dragging = $state(false)

  function setDragging(next: boolean) {
    dragging = next
    const root = document.documentElement
    if (next) root.dataset.paneDrag = 'horizontal'
    else delete root.dataset.paneDrag
  }

  function pointerDown(event: PointerEvent) {
    if (event.button !== 0) return
    event.preventDefault()
    ;(event.currentTarget as Element).setPointerCapture(event.pointerId)
    setDragging(true)
  }

  function pointerMove(event: PointerEvent) {
    if (!dragging) return
    const left = aside?.getBoundingClientRect().left ?? 0
    width = clampSidebarWidth(event.clientX - left)
  }

  function pointerUp() {
    if (!dragging) return
    setDragging(false)
    writeSidebarWidth(width)
  }

  function keyDown(event: KeyboardEvent) {
    const step =
      event.key === 'ArrowLeft'
        ? -KEY_STEP
        : event.key === 'ArrowRight'
          ? KEY_STEP
          : 0
    if (!step) return
    event.preventDefault()
    const current = width ?? aside?.getBoundingClientRect().width ?? 0
    width = clampSidebarWidth(current + step)
    writeSidebarWidth(width)
  }

  function reset() {
    width = null
    writeSidebarWidth(null)
  }
</script>

<div
  class="app-shell relative grid h-dvh min-h-0 overflow-hidden bg-background text-foreground"
  style:--sidebar-w={width == null ? undefined : `${width}px`}
>
  <aside
    bind:this={aside}
    class="relative flex min-h-0 min-w-0 flex-col border-r border-sidebar-border bg-sidebar"
  >
    <Sidebar />
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="group/handle absolute inset-y-0 -right-1.5 z-20 w-3 cursor-col-resize outline-none"
      role="separator"
      tabindex="0"
      aria-orientation="vertical"
      aria-label="Resize sidebar"
      aria-valuemin={SIDEBAR_MIN_WIDTH}
      aria-valuemax={SIDEBAR_MAX_WIDTH}
      aria-valuenow={width ?? undefined}
      title="Drag to resize, double-click to reset"
      data-od-id="sidebar-resize-handle"
      onpointerdown={pointerDown}
      onpointermove={pointerMove}
      onpointerup={pointerUp}
      onpointercancel={pointerUp}
      onkeydown={keyDown}
      ondblclick={reset}
    >
      <span
        class="absolute inset-y-0 left-1/2 w-px -translate-x-1/2 transition-[background-color,box-shadow] group-hover/handle:bg-primary group-hover/handle:ring-[1.5px] group-hover/handle:ring-primary group-focus-visible/handle:bg-primary {dragging
          ? 'bg-primary ring-[1.5px] ring-primary'
          : ''}"
        aria-hidden="true"
      ></span>
    </div>
  </aside>
  <main class="main-area flex min-h-0 min-w-0 flex-col">
    {@render children()}
  </main>
</div>
