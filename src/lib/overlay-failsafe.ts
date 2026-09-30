/**
 * Safety net for bits-ui layers that never finish closing.
 *
 * While a dialog, menu, popover or select closes, bits-ui marks it `data-ending-style` and waits
 * for its exit animation before unmounting it and, for dialogs, restoring the body's
 * `pointer-events`. On some release installs WebKit never reports that animation finished, which
 * left an invisible overlay over the app and the body ignoring the mouse. If a layer is still
 * closing after STUCK_MS, finish its animations, stop it taking clicks, and unblock the body
 * once no modal is open.
 */
const STUCK_MS = 1000
const ENDING = 'data-ending-style'
const OPEN_MODAL =
  '[role="dialog"][data-state="open"], [role="alertdialog"][data-state="open"]'

export function installOverlayFailsafe(
  root: HTMLElement = document.body,
): () => void {
  const timers = new Map<Element, number>()
  const disabled = new Set<HTMLElement>()

  function rescue(el: HTMLElement) {
    timers.delete(el)
    if (!el.isConnected || !el.hasAttribute(ENDING)) return
    for (const animation of el.getAnimations?.() ?? []) {
      try {
        animation.finish()
      } catch {
        // An infinite animation can't be finished; the pointer-events fallback covers it.
      }
    }
    // If finishing the animation let bits-ui unmount the layer, there is nothing left to do.
    if (el.isConnected && el.hasAttribute(ENDING)) {
      el.style.pointerEvents = 'none'
      disabled.add(el)
    }
    if (
      document.body.style.pointerEvents === 'none' &&
      !document.querySelector(OPEN_MODAL)
    ) {
      document.body.style.removeProperty('pointer-events')
    }
  }

  function watch(el: Element) {
    if (!(el instanceof HTMLElement)) return
    if (el.hasAttribute(ENDING)) {
      if (!timers.has(el))
        timers.set(
          el,
          window.setTimeout(() => rescue(el), STUCK_MS),
        )
      return
    }
    // The layer reopened or settled: cancel the rescue and give its clicks back.
    const timer = timers.get(el)
    if (timer !== undefined) window.clearTimeout(timer)
    timers.delete(el)
    if (disabled.delete(el)) el.style.removeProperty('pointer-events')
  }

  const observer = new MutationObserver((records) => {
    for (const record of records) {
      if (record.type === 'attributes') watch(record.target as Element)
    }
  })
  observer.observe(root, {
    subtree: true,
    attributes: true,
    attributeFilter: [ENDING],
  })

  return () => {
    observer.disconnect()
    for (const timer of timers.values()) window.clearTimeout(timer)
    timers.clear()
  }
}
