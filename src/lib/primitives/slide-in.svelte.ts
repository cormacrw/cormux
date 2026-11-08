import { untrack } from 'svelte'

/**
 * A panel mounted on its first open and kept, so reopening doesn't rebuild it. That first open
 * mounts it closed and opens it two frames later, once the browser has drawn it closed, so the
 * CSS transition slides it in like every later open. A new `owner` shows it as it stands, at once.
 * Call while a component initialises.
 */
export function slideIn(visible: () => boolean, owner: () => unknown) {
  let mounted = $state(false)
  let armed = $state(false)
  $effect(() => {
    if (!visible() || untrack(() => mounted)) return
    mounted = true
    requestAnimationFrame(() => requestAnimationFrame(() => (armed = true)))
  })
  $effect(() => {
    void owner()
    untrack(() => {
      mounted = armed = visible()
    })
  })
  return {
    get mounted() {
      return mounted
    },
    get open() {
      return visible() && armed
    },
  }
}
