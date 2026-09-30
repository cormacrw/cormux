import { settings } from '$lib/state/settings.svelte'

/**
 * Duration for a Svelte transition. They run in JS, so the reduce-motion CSS rule doesn't
 * reach them: this returns 0 under the in-app switch or the OS setting.
 */
export function motionMs(duration: number): number {
  return settings.reduceMotion ||
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
    ? 0
    : duration
}
