/**
 * True under the in-app reduce-motion switch or the OS setting. The switch is read from the
 * `reduce-motion` class App keeps on <html> from Settings, which app.css's rules follow too.
 */
export function prefersReducedMotion(): boolean {
  const doc = globalThis.document
  if (!doc) return false
  return (
    doc.documentElement.classList.contains('reduce-motion') ||
    globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches
  )
}

/**
 * Duration for a Svelte transition. They run in JS, so the reduce-motion CSS rule doesn't
 * reach them: this returns 0 when motion is reduced.
 */
export function motionMs(duration: number): number {
  return prefersReducedMotion() ? 0 : duration
}
