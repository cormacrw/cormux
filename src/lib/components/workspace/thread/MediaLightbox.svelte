<script lang="ts" module>
  /** Where the lightbox mounts: the workspace column, beside the sidebar. */
  export const MEDIA_HOST_ATTR = 'data-media-lightbox-host'
</script>

<script lang="ts">
  import Maximize from '@lucide/svelte/icons/maximize'
  import X from '@lucide/svelte/icons/x'
  import ZoomIn from '@lucide/svelte/icons/zoom-in'
  import ZoomOut from '@lucide/svelte/icons/zoom-out'

  let {
    src,
    alt,
    name,
    onClose,
  }: {
    src: string
    alt: string
    name: string
    onClose: () => void
  } = $props()

  const MIN_SCALE = 1
  const MAX_SCALE = 8
  const STEP = 1.5

  let scale = $state(1)
  let tx = $state(0)
  let ty = $state(0)
  let stage = $state<HTMLDivElement>()
  let image = $state<HTMLImageElement>()
  let drag: { x: number; y: number; tx: number; ty: number } | null = null
  let dragged = false
  let dragging = $state(false)

  /**
   * Moves the overlay out of the timeline into the workspace column, so it
   * sits above the workspace header but stops at the sidebar. The timeline is
   * inside a view-transition stacking context, which traps `fixed` children
   * under the header.
   */
  function portal(node: HTMLElement) {
    const host = document.querySelector(`[${MEDIA_HOST_ATTR}]`)
    if (!host) {
      node.classList.replace('absolute', 'fixed')
      return
    }
    host.appendChild(node)
    return () => node.remove()
  }

  function focusOnMount(node: HTMLElement) {
    node.focus()
  }

  function clampPan(nextScale: number, x: number, y: number) {
    if (!stage || !image) return { x, y }
    const maxX = Math.max(
      0,
      (image.offsetWidth * nextScale - stage.clientWidth) / 2,
    )
    const maxY = Math.max(
      0,
      (image.offsetHeight * nextScale - stage.clientHeight) / 2,
    )
    return {
      x: Math.min(maxX, Math.max(-maxX, x)),
      y: Math.min(maxY, Math.max(-maxY, y)),
    }
  }

  /** Zooms to `next`, keeping the point under (clientX, clientY) still. */
  function zoomTo(next: number, clientX?: number, clientY?: number) {
    const target = Math.min(MAX_SCALE, Math.max(MIN_SCALE, next))
    let px = 0
    let py = 0
    if (stage && clientX !== undefined && clientY !== undefined) {
      const rect = stage.getBoundingClientRect()
      px = clientX - (rect.left + rect.width / 2)
      py = clientY - (rect.top + rect.height / 2)
    }
    const ratio = target / scale
    const pan = clampPan(target, px - (px - tx) * ratio, py - (py - ty) * ratio)
    scale = target
    tx = pan.x
    ty = pan.y
  }

  function reset() {
    scale = 1
    tx = 0
    ty = 0
  }

  function onWheel(event: WheelEvent) {
    event.preventDefault()
    // A trackpad pinch arrives as a wheel event with ctrlKey set.
    if (event.ctrlKey || event.metaKey || scale === 1) {
      zoomTo(
        scale * Math.exp(-event.deltaY * 0.01),
        event.clientX,
        event.clientY,
      )
      return
    }
    const pan = clampPan(scale, tx - event.deltaX, ty - event.deltaY)
    tx = pan.x
    ty = pan.y
  }

  /** Svelte binds `onwheel` passively, so preventDefault would be ignored. */
  function wheel(node: HTMLElement) {
    node.addEventListener('wheel', onWheel, { passive: false })
    return () => node.removeEventListener('wheel', onWheel)
  }

  function onPointerDown(event: PointerEvent) {
    if (event.button !== 0) return
    dragged = false
    if (scale === 1) return
    drag = { x: event.clientX, y: event.clientY, tx, ty }
    dragging = true
    ;(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId)
  }

  function onPointerMove(event: PointerEvent) {
    if (!drag) return
    const dx = event.clientX - drag.x
    const dy = event.clientY - drag.y
    if (Math.abs(dx) + Math.abs(dy) > 3) dragged = true
    const pan = clampPan(scale, drag.tx + dx, drag.ty + dy)
    tx = pan.x
    ty = pan.y
  }

  function onPointerUp() {
    drag = null
    dragging = false
  }

  function onStageClick(event: MouseEvent) {
    // A click on the backdrop closes; a click that ended a pan does not.
    if (dragged) return
    if (event.target === event.currentTarget) onClose()
  }

  function onDoubleClick(event: MouseEvent) {
    if (scale > 1) reset()
    else zoomTo(2.5, event.clientX, event.clientY)
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.stopPropagation()
      onClose()
    } else if (event.key === '+' || event.key === '=') {
      event.preventDefault()
      zoomTo(scale * STEP)
    } else if (event.key === '-') {
      event.preventDefault()
      zoomTo(scale / STEP)
    } else if (event.key === '0') {
      event.preventDefault()
      reset()
    }
  }

  const percent = $derived(`${Math.round(scale * 100)}%`)
  const cursor = $derived(
    dragging ? 'cursor-grabbing' : scale > 1 ? 'cursor-grab' : 'cursor-zoom-in',
  )
</script>

<div
  {@attach portal}
  {@attach focusOnMount}
  class="absolute inset-0 z-50 flex flex-col bg-black/90 outline-none backdrop-blur-md"
  role="dialog"
  aria-modal="true"
  aria-label={name}
  tabindex="-1"
  onkeydown={onKeydown}
>
  <div
    class="flex shrink-0 items-center justify-between gap-3 px-4 py-2 text-white/80"
  >
    <span class="min-w-0 truncate text-xs" title={name}>{name}</span>
    <div class="flex items-center gap-1">
      <button
        type="button"
        class="inline-flex size-8 items-center justify-center rounded-full hover:bg-white/15 disabled:opacity-40"
        aria-label="Zoom out"
        disabled={scale <= MIN_SCALE}
        onclick={() => zoomTo(scale / STEP)}
      >
        <ZoomOut class="size-4" aria-hidden="true" />
      </button>
      <button
        type="button"
        class="min-w-12 rounded-md px-1.5 py-1 text-xs tabular-nums hover:bg-white/15"
        aria-label="Reset zoom"
        title="Fit to view (0)"
        onclick={reset}
      >
        {percent}
      </button>
      <button
        type="button"
        class="inline-flex size-8 items-center justify-center rounded-full hover:bg-white/15 disabled:opacity-40"
        aria-label="Zoom in"
        disabled={scale >= MAX_SCALE}
        onclick={() => zoomTo(scale * STEP)}
      >
        <ZoomIn class="size-4" aria-hidden="true" />
      </button>
      <button
        type="button"
        class="inline-flex size-8 items-center justify-center rounded-full hover:bg-white/15"
        aria-label="Fit to view"
        onclick={reset}
      >
        <Maximize class="size-4" aria-hidden="true" />
      </button>
      <span class="mx-1 h-4 w-px bg-white/20" aria-hidden="true"></span>
      <button
        type="button"
        class="inline-flex size-8 items-center justify-center rounded-full bg-white/10 hover:bg-white/20"
        aria-label="Close"
        onclick={onClose}
      >
        <X class="size-4" aria-hidden="true" />
      </button>
    </div>
  </div>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    bind:this={stage}
    class="relative flex min-h-0 flex-1 items-center justify-center overflow-hidden p-6 pt-0 select-none"
    onclick={onStageClick}
    {@attach wheel}
  >
    <img
      bind:this={image}
      {src}
      {alt}
      draggable="false"
      class="max-h-full max-w-full rounded-lg object-contain {cursor} {dragging
        ? ''
        : 'transition-transform duration-100 ease-out'}"
      style:transform="translate({tx}px, {ty}px) scale({scale})"
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
      ondblclick={onDoubleClick}
    />
  </div>
</div>
