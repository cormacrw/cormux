import { tick } from 'svelte'
import { prefersReducedMotion } from '$lib/motion'

/** What a thread shows of its conversation: the intro or greeting, and the timeline. */
const PARTS = '#thread-scroll [data-conversation-part]'
const EASE = 'cubic-bezier(0.2, 0.8, 0.2, 1)'

/**
 * Fades the conversation on screen up and away before it's swapped for
 * another (a new session, or resuming one). Resolves once it's gone, with a
 * function that brings it back if the swap doesn't happen.
 */
export async function leaveConversation(): Promise<() => void> {
  if (prefersReducedMotion()) return () => {}
  const animations = [...document.querySelectorAll<HTMLElement>(PARTS)].map(
    (part) =>
      part.animate(
        [
          { opacity: 1, transform: 'none' },
          { opacity: 0, transform: 'translate3d(0, -10px, 0)' },
        ],
        { duration: 160, easing: 'ease-in', fill: 'forwards' },
      ),
  )
  await Promise.all(animations.map((animation) => animation.finished))
  return () => animations.forEach((animation) => animation.cancel())
}

/** Rises the conversation that just arrived into place, clearing any exit. */
export function enterConversation() {
  for (const part of document.querySelectorAll<HTMLElement>(PARTS)) {
    for (const animation of part.getAnimations()) animation.cancel()
    if (prefersReducedMotion()) continue
    part.animate(
      [
        { opacity: 0, transform: 'translate3d(0, 12px, 0)' },
        { opacity: 1, transform: 'none' },
      ],
      { duration: 260, easing: EASE },
    )
  }
}

/**
 * The first prompt in an empty conversation: the greeting falls away as the
 * message pops in (styles in app.css). Plain update where unsupported.
 */
export function firstPromptTransition(update: () => void) {
  const greeting = document.querySelector('[data-od-id="thread-hero"]')
  if (
    !greeting ||
    prefersReducedMotion() ||
    typeof document.startViewTransition !== 'function'
  ) {
    update()
    return
  }
  const transition = document.startViewTransition(async () => {
    update()
    await tick()
  })
  // Cut short by another transition, the update still lands.
  transition.ready.catch(() => {})
  transition.finished.catch(() => {})
}
