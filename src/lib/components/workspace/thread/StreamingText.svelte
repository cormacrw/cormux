<script lang="ts">
  import { onDestroy, untrack } from 'svelte'
  import { prefersReducedMotion, type EntryScope } from '$lib/thread/entering'
  import ThoughtMarkdown from './ThoughtMarkdown.svelte'

  let {
    text,
    entries,
    key,
    live = false,
  }: {
    text: string
    entries: EntryScope | undefined
    key: string
    /** The agent is still writing this reply, so the cursor stays after it catches up. */
    live?: boolean
  } = $props()

  // TextType's pacing: one character per tick at a jittered speed so it reads as typed
  // (~35 chars/s). A reply far ahead speeds it up, but never past MAX_CHARS_PER_SEC, so long
  // replies trail the agent rather than racing to keep up with it.
  const SPEED_MS = { min: 18, max: 40 }
  const MAX_CHARS_PER_SEC = 60
  const MAX_LAG_SEC = 3

  // Replies that were already in the thread when it opened show in full.
  let shown = $state(
    untrack(() =>
      entries?.claim(key) && !prefersReducedMotion() ? 0 : text.length,
    ),
  )
  let timer: ReturnType<typeof setTimeout> | undefined
  let lastTickAt = 0
  // Fractional characters earned but not yet shown, so the pace holds between ticks.
  let owed = 0

  const typing = $derived(shown < text.length)

  function nextDelay() {
    return Math.random() * (SPEED_MS.max - SPEED_MS.min) + SPEED_MS.min
  }

  function tick() {
    timer = undefined
    const remaining = text.length - shown
    if (remaining <= 0 || prefersReducedMotion()) {
      shown = text.length
      return
    }
    // Measured, not assumed: a timer that fires late (a busy or backgrounded page) reveals
    // what it owes, so the reply still keeps pace.
    const now = performance.now()
    const elapsedMs = Math.min(now - lastTickAt, 1000)
    lastTickAt = now
    const averageMs = (SPEED_MS.min + SPEED_MS.max) / 2
    const pace = 1000 / averageMs
    const perSec = Math.min(
      MAX_CHARS_PER_SEC,
      Math.max(pace, remaining / MAX_LAG_SEC),
    )
    owed += (perSec * elapsedMs) / 1000
    const step = Math.floor(owed)
    owed -= step
    shown = Math.min(text.length, shown + step)
    timer = setTimeout(tick, nextDelay())
  }

  $effect(() => {
    const length = text.length
    untrack(() => {
      if (length <= shown || timer) return
      lastTickAt = performance.now()
      owed = 1
      timer = setTimeout(tick, nextDelay())
    })
  })

  onDestroy(() => clearTimeout(timer))
</script>

<ThoughtMarkdown text={text.slice(0, shown)} cursor={typing || live} />
