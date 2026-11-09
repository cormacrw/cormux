import { getCurrentWindow } from '@tauri-apps/api/window'

/**
 * `use:windowDrag` on a title-bar area: pressing on its empty space drags the window, while its
 * buttons, links and fields keep their clicks.
 */
export function windowDrag(node: HTMLElement) {
  const onMouseDown = (event: MouseEvent) => {
    if (event.button !== 0) return
    const target = event.target
    if (!(target instanceof Element)) return
    if (target.closest('button, a, input, textarea')) return
    void getCurrentWindow().startDragging()
  }
  node.addEventListener('mousedown', onMouseDown)
  return {
    destroy: () => node.removeEventListener('mousedown', onMouseDown),
  }
}
