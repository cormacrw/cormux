import { settings } from '$lib/state/settings.svelte'

/** True under the in-app reduce-motion switch or the OS setting. */
export function prefersReducedMotion(): boolean {
  return (
    settings.reduceMotion ||
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  )
}

/**
 * Duration for a Svelte transition. They run in JS, so the reduce-motion CSS rule doesn't
 * reach them: this returns 0 when motion is reduced.
 */
export function motionMs(duration: number): number {
  return prefersReducedMotion() ? 0 : duration
}
