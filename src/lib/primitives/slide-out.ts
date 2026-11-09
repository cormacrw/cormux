import { readPref, writePref } from './local-pref'

/** A pane width saved under `key`, or `fallback` when none is saved (or it's under `min`). */
export function readPaneWidth(key: string, min: number, fallback: number) {
  const stored = Number(readPref(key))
  return stored >= min ? stored : fallback
}

export function savePaneWidth(key: string, width: number) {
  writePref(key, String(width))
}

type ResizeEdgeOptions = {
  /** The area the pane sits at the right of. */
  bounds: () => DOMRect | undefined
  min: number
  /** Room the pane always leaves to its left. */
  reserve: number
  onResize: (width: number) => void
  onEnd?: () => void
}

/** `use:resizeEdge` on a pane's left edge: dragging it resizes the pane. */
export function resizeEdge(node: HTMLElement, initial: ResizeEdgeOptions) {
  let options = initial
  const onPointerDown = (event: PointerEvent) => {
    if (event.button !== 0) return
    const area = options.bounds()
    if (!area) return
    event.preventDefault()
    node.setPointerCapture(event.pointerId)
    const { min, reserve } = options
    const move = (next: PointerEvent) => {
      const width = area.right - 12 - next.clientX
      options.onResize(
        Math.round(
          Math.min(Math.max(width, min), Math.max(min, area.width - reserve)),
        ),
      )
    }
    const up = () => {
      node.removeEventListener('pointermove', move)
      node.removeEventListener('pointerup', up)
      node.removeEventListener('pointercancel', up)
      options.onEnd?.()
    }
    node.addEventListener('pointermove', move)
    node.addEventListener('pointerup', up)
    node.addEventListener('pointercancel', up)
  }
  node.addEventListener('pointerdown', onPointerDown)
  return {
    update: (next: ResizeEdgeOptions) => (options = next),
    destroy: () => node.removeEventListener('pointerdown', onPointerDown),
  }
}
