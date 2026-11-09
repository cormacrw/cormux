import { getContext, setContext, tick } from 'svelte'
import type { Action } from 'svelte/action'
import { prefersReducedMotion } from '$lib/motion'

/** What a thread shows of its conversation, which moves as one: the intro or greeting, and the timeline. */
const parts = new Set<HTMLElement>()
/** The greeting on screen, the only part the first prompt's view transition snapshots. */
const greetings = new Set<HTMLElement>()
const EASE = 'cubic-bezier(0.2, 0.8, 0.2, 1)'

const IN_CONVERSATION = Symbol('in-conversation')

function trackIn(set: Set<HTMLElement>): Action<HTMLElement> {
  return (node) => {
    set.add(node)
    return { destroy: () => void set.delete(node) }
  }
}

/**
 * For the conversation's component to call as it starts: what renders inside it can then join
 * it with `conversationPart`. Returns the action for its own parts.
 */
export function provideConversation(): Action<HTMLElement> {
  setContext(IN_CONVERSATION, true)
  return trackIn(parts)
}

/**
 * For a component to call as it starts: an action that makes its element part of the
 * conversation it renders in, and nothing outside one.
 */
export function conversationPart(): Action<HTMLElement> {
  return getContext<boolean>(IN_CONVERSATION) ? trackIn(parts) : () => {}
}

/** Marks the greeting, which a first prompt sends falling away. */
export const greetingPart: Action<HTMLElement> = trackIn(greetings)

/**
 * Fades the conversation on screen up and away before it's swapped for
 * another (a new session, or resuming one). Resolves once it's gone, with a
 * function that brings it back if the swap doesn't happen.
 */
export async function leaveConversation(): Promise<() => void> {
  if (prefersReducedMotion()) return () => {}
  const animations = [...parts].map((part) =>
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
  for (const part of parts) {
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
 * message pops in (styles in app.css). Plain update where unsupported. An update
 * that returns a promise holds the old screen until it settles. `onGreetingLeaves`
 * runs as the greeting starts to go, for whatever goes with it; `onLanded` once the
 * update is on the page, for motion on what it brought.
 */
export function firstPromptTransition(
  update: () => void | Promise<void>,
  onGreetingLeaves: () => void,
  onLanded?: () => void,
) {
  const land = async () => {
    await update()
    await tick()
  }
  const greeting = greetings.size > 0
  if (greeting) onGreetingLeaves()
  if (
    !greeting ||
    prefersReducedMotion() ||
    typeof document.startViewTransition !== 'function'
  ) {
    void land().then(onLanded)
    return
  }
  // Only the greeting is snapshotted; the rest of the page stays live, so the
  // backdrop's wordmark sinks away smoothly underneath.
  const root = document.documentElement
  root.classList.add('first-prompt')
  const transition = document.startViewTransition(land)
  // The update has landed by then even when another transition cut this one short.
  void transition.updateCallbackDone.then(onLanded)
  transition.ready.catch(() => {})
  transition.finished
    .catch(() => {})
    .finally(() => root.classList.remove('first-prompt'))
}
