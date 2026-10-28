<script lang="ts" module>
  // Only one reply types at a time. Each one gets an order as it mounts; when a newer reply
  // starts typing, older ones show in full, and an older one that gets more text while a
  // newer one types just shows it, so two replies never type side by side.
  // Plain module state, not reactive: nothing renders it.
  let lastOrder = 0
  const typing: Record<number, () => void> = {}

  function takeOrder(): number {
    lastOrder += 1
    return lastOrder
  }

  function startTyping(order: number, finish: () => void): boolean {
    const others = Object.keys(typing).map(Number)
    if (others.some((other) => other > order)) return false
    for (const other of others) {
      const finishOther = typing[other]
      delete typing[other]
      finishOther?.()
    }
    typing[order] = finish
    return true
  }

  function stopTyping(order: number) {
    delete typing[order]
  }

  // Replies already shown, by text. Rebuilding a thread from the store can
  // give a reply a new row id; this keeps it from typing out a second time.
  const shownTexts: string[] = []
  const SHOWN_LIMIT = 300

  function rememberShown(text: string) {
    if (text.length < 2) return
    const at = shownTexts.findIndex(
      (seen) => text.startsWith(seen) || seen.startsWith(text),
    )
    if (at >= 0) {
      if (shownTexts[at]!.length >= text.length) return
      shownTexts.splice(at, 1)
    }
    shownTexts.push(text)
    if (shownTexts.length > SHOWN_LIMIT) shownTexts.shift()
  }

  /** How much of `text` was shown before, under any id. */
  function alreadyShown(text: string): number {
    let best = 0
    for (const seen of shownTexts) {
      if (seen.startsWith(text)) return text.length
      if (text.startsWith(seen)) best = Math.max(best, seen.length)
    }
    return best
  }
</script>

<script lang="ts">
  import { onDestroy, untrack } from 'svelte'
  import { prefersReducedMotion } from '$lib/motion'
  import type { EntryScope } from '$lib/thread/entering'
  import { revealEnd, revealRate, stableMarkdown } from '$lib/thread/typing'
  import ThoughtMarkdown from './ThoughtMarkdown.svelte'

  let {
    text,
    entries,
    key,
    live = false,
    workdir,
  }: {
    text: string
    entries: EntryScope | undefined
    key: string
    /** The agent is still writing this reply, so the cursor stays after it catches up. */
    live?: boolean
    /** The thread's worktree, so media the reply references can resolve and render. */
    workdir?: string
  } = $props()

  const order = takeOrder()

  // Replies that were already in the thread when it opened show in full, and
  // one seen before under another id picks up where it left off.
  let shown = $state(
    untrack(() =>
      entries?.claim(key) && !prefersReducedMotion()
        ? alreadyShown(text)
        : text.length,
    ),
  )
  $effect(() => {
    const visible = text.slice(0, shown)
    untrack(() => rememberShown(visible))
  })
  let frame = 0
  let lastFrameAt = 0
  // Characters earned but not yet shown, so the pace holds between frames.
  let owed = 0

  const isTyping = $derived(shown < text.length)
  const markdown = $derived(
    isTyping ? stableMarkdown(text.slice(0, shown)) : text,
  )

  function finish() {
    cancelAnimationFrame(frame)
    frame = 0
    shown = text.length
  }

  function step(now: number) {
    frame = 0
    const remaining = text.length - shown
    if (remaining <= 0 || prefersReducedMotion()) {
      shown = text.length
      stopTyping(order)
      return
    }
    // A frame that comes late (a busy or hidden page) reveals what it owes, capped so a
    // long pause doesn't dump the whole reply at once.
    const elapsedMs = Math.min(now - lastFrameAt, 250)
    lastFrameAt = now
    owed += (revealRate(remaining, live) * elapsedMs) / 1000
    const chars = Math.floor(owed)
    if (chars > 0) {
      const next = revealEnd(text, shown, chars)
      owed = Math.max(0, owed - (next - shown))
      shown = next
    }
    frame = requestAnimationFrame(step)
  }

  $effect(() => {
    const length = text.length
    untrack(() => {
      if (length <= shown || frame) return
      if (!startTyping(order, finish)) {
        shown = length
        return
      }
      lastFrameAt = performance.now()
      owed = 0
      frame = requestAnimationFrame(step)
    })
  })

  onDestroy(() => {
    cancelAnimationFrame(frame)
    stopTyping(order)
  })
</script>

<ThoughtMarkdown text={markdown} cursor={isTyping || live} media {workdir} />
