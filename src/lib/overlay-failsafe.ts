/**
 * Safety net for bits-ui layers that leave the app ignoring the mouse.
 *
 * Dialogs, menus, popovers and selects set `pointer-events: none` on the body while open
 * and restore it once they close. On some release installs that goes wrong in two ways:
 *
 * - WebKit never reports a closing layer's exit animation as finished, so bits-ui keeps it
 *   mounted (`data-ending-style`) with its overlay over the app. After STUCK_MS, finish its
 *   animations and stop it taking clicks.
 * - The layer closes, but the body keeps `pointer-events: none`. The keyboard still works,
 *   the mouse does nothing, not even hover. Whenever the body or a layer changes, check
 *   again after SETTLE_MS and clear the lock if nothing that sets it is still open.
 */
const STUCK_MS = 1000
const SETTLE_MS = 300
const ENDING = 'data-ending-style'
// Every bits-ui layer that locks the body renders one of these while it's open.
const OPEN_LAYER = ['dialog', 'alertdialog', 'menu', 'listbox']
  .map((role) => `[role="${role}"][data-state="open"]`)
  .join(', ')

export function installOverlayFailsafe(
  root: HTMLElement = document.body,
): () => void {
  const timers = new Map<Element, number>()
  const disabled = new Set<HTMLElement>()
  let settleTimer: number | undefined

  function unlockBodyIfIdle() {
    settleTimer = undefined
    if (
      document.body.style.pointerEvents === 'none' &&
      !document.querySelector(OPEN_LAYER)
    ) {
      document.body.style.removeProperty('pointer-events')
    }
  }

  // Not a debounce: the DOM changes constantly while an agent streams, so a check that
  // restarted on every change might never run.
  function scheduleBodyCheck() {
    if (settleTimer === undefined)
      settleTimer = window.setTimeout(unlockBodyIfIdle, SETTLE_MS)
  }

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
    unlockBodyIfIdle()
  }

  function watchEnding(el: Element) {
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
    let layersChanged = false
    for (const record of records) {
      if (record.attributeName === ENDING) {
        watchEnding(record.target as Element)
      } else if (record.attributeName === 'data-state') {
        layersChanged = true
      } else if (record.target === document.body) {
        // The body's own style, or a portal mounting or unmounting under it.
        layersChanged = true
      }
    }
    if (layersChanged && document.body.style.pointerEvents === 'none')
      scheduleBodyCheck()
  })
  observer.observe(root, {
    subtree: true,
    childList: true,
    attributes: true,
    attributeFilter: [ENDING, 'data-state', 'style'],
  })

  return () => {
    observer.disconnect()
    for (const timer of timers.values()) window.clearTimeout(timer)
    timers.clear()
    if (settleTimer !== undefined) window.clearTimeout(settleTimer)
  }
}
