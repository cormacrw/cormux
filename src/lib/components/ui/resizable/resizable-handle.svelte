<script lang="ts">
  import * as ResizablePrimitive from 'paneforge'
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    withHandle = false,
    onDraggingChange,
    ...restProps
  }: WithoutChildrenOrChild<ResizablePrimitive.PaneResizerProps> & {
    withHandle?: boolean
  } = $props()

  // Marks <html> mid-drag so app.css can hold the resize cursor everywhere. An attribute
  // on the root, rather than `body:has(...)`, which made every DOM change restyle the page.
  function draggingChange(dragging: boolean) {
    const root = document.documentElement
    if (dragging) root.dataset.paneDrag = ref?.dataset.direction ?? 'horizontal'
    else delete root.dataset.paneDrag
    onDraggingChange?.(dragging)
  }
</script>

<ResizablePrimitive.PaneResizer
  bind:ref
  onDraggingChange={draggingChange}
  data-slot="resizable-handle"
  class={cn(
    'group/handle relative z-10 flex w-px items-center justify-center bg-border transition-[background-color,box-shadow] hover:bg-primary hover:ring-[1.5px] hover:ring-primary focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:outline-hidden data-[active=pointer]:bg-primary data-[active=pointer]:ring-[1.5px] data-[active=pointer]:ring-primary data-[direction=vertical]:h-px data-[direction=vertical]:w-full [&[data-direction=vertical]>div]:rotate-90',
    className,
  )}
  {...restProps}
>
  <!-- Grab area, wider than the 1px line. -->
  <span
    class="absolute inset-y-0 left-1/2 w-3 -translate-x-1/2 cursor-col-resize group-data-[direction=vertical]/handle:inset-x-0 group-data-[direction=vertical]/handle:inset-y-auto group-data-[direction=vertical]/handle:top-1/2 group-data-[direction=vertical]/handle:left-0 group-data-[direction=vertical]/handle:h-3 group-data-[direction=vertical]/handle:w-full group-data-[direction=vertical]/handle:translate-x-0 group-data-[direction=vertical]/handle:-translate-y-1/2 group-data-[direction=vertical]/handle:cursor-row-resize"
    aria-hidden="true"
  ></span>
  {#if withHandle}
    <div
      class="pointer-events-none z-10 flex h-6 w-1 shrink-0 rounded-lg bg-border transition-colors group-hover/handle:bg-primary group-data-[active=pointer]/handle:bg-primary"
    ></div>
  {/if}
</ResizablePrimitive.PaneResizer>
