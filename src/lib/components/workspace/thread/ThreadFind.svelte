<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import ChevronUp from '@lucide/svelte/icons/chevron-up'
  import X from '@lucide/svelte/icons/x'
  import { Button } from '$lib/components/ui/button'
  import { threadFind } from '$lib/state/thread-find.svelte'
  import {
    findCounterText,
    findMatches,
    rowKey,
    stepMatch,
  } from '$lib/thread/find'
  import type { TimelineRow } from '$lib/thread/timeline-types'

  let {
    rows,
    contentEl,
    scrollToRow,
    onScrollAway,
  }: {
    rows: TimelineRow[]
    /** The rendered conversation, where matches are highlighted. */
    contentEl: HTMLElement | null | undefined
    /** Brings a row on screen when the list only renders some (long threads). */
    scrollToRow?: (index: number) => void
    /** Called before moving to a match, so following the newest message doesn't pull back. */
    onScrollAway?: () => void
  } = $props()

  let open = $state(false)
  let query = $state('')
  let current = $state(0)
  let inputEl = $state<HTMLInputElement | null>(null)
  let returnFocus: HTMLElement | null = null

  const matches = $derived(open ? findMatches(rows, query) : [])
  const counter = $derived(
    query.trim() ? findCounterText(current, matches.length) : '',
  )

  // ⌘F opens the bar, or selects what's in it when it's already open. The first run only
  // records where the counter starts.
  let seenRequests = threadFind.requests
  $effect(() => {
    const requests = threadFind.requests
    if (requests === seenRequests) return
    seenRequests = requests
    untrack(() => {
      if (!open) {
        returnFocus = document.activeElement as HTMLElement | null
        open = true
      }
      void tick().then(() => {
        inputEl?.focus()
        inputEl?.select()
      })
    })
  })

  // A new search starts at the newest match, where the reader usually is.
  $effect(() => {
    void query
    untrack(() => {
      current = Math.max(0, findMatches(rows, query).length - 1)
    })
  })

  // Rows stream in while the bar is open, so keep the current match in range.
  $effect(() => {
    const total = matches.length
    untrack(() => {
      if (current >= total) current = Math.max(0, total - 1)
    })
  })

  // Streaming rows re-run this to keep highlights fresh, but only a new search or match
  // scrolls, so the reader isn't pulled back on every chunk.
  let revealed = ''
  $effect(() => {
    const match = matches[current]
    const needle = query.trim()
    const root = contentEl
    if (!open || !root) return
    const target = `${current}\u0000${needle}`
    const scroll = target !== revealed
    revealed = target
    const show = () =>
      void reveal(
        root,
        needle,
        match ? rowKey(rows[match.row]!) : null,
        match,
        scroll,
      )
    clearTimeout(refreshTimer)
    if (scroll) show()
    else refreshTimer = setTimeout(show, REFRESH_MS)
  })

  // Redrawing every highlight walks the whole page, so streaming only does it a few times a second.
  const REFRESH_MS = 250
  let refreshTimer: ReturnType<typeof setTimeout> | undefined

  onDestroy(() => {
    clearTimeout(refreshTimer)
    clearHighlights()
  })

  function clearHighlights() {
    if (typeof CSS === 'undefined' || !CSS.highlights) return
    CSS.highlights.delete('thread-find')
    CSS.highlights.delete('thread-find-current')
  }

  function textRanges(root: Element, needle: string): Range[] {
    const lower = needle.toLocaleLowerCase()
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
    const ranges: Range[] = []
    while (walker.nextNode()) {
      const node = walker.currentNode as Text
      const text = node.data.toLocaleLowerCase()
      let at = text.indexOf(lower)
      while (at !== -1) {
        const range = document.createRange()
        range.setStart(node, at)
        range.setEnd(node, at + needle.length)
        ranges.push(range)
        at = text.indexOf(lower, at + needle.length)
      }
    }
    return ranges
  }

  async function reveal(
    root: HTMLElement,
    needle: string,
    key: string | null,
    match: { row: number; occurrence: number } | undefined,
    scroll: boolean,
  ) {
    if (!needle || !match || key === null) {
      clearHighlights()
      return
    }
    if (scroll) onScrollAway?.()
    let rowEl = root.querySelector(`[data-row-key="${CSS.escape(key)}"]`)
    if (!rowEl && scroll && scrollToRow) {
      // A long thread only renders rows near the screen; bring this one in, then look again.
      scrollToRow(match.row)
      for (let frame = 0; frame < 10 && !rowEl; frame += 1) {
        await new Promise((done) => requestAnimationFrame(done))
        rowEl = root.querySelector(`[data-row-key="${CSS.escape(key)}"]`)
      }
    }
    // Folded thoughts and runs aren't rendered until opened, and that happens on `toggle`.
    const folds = rowEl?.querySelectorAll<HTMLDetailsElement>(
      'details[data-lazy-fold]:not([open])',
    )
    if (folds?.length) {
      for (const fold of folds) fold.open = true
      await new Promise((done) => requestAnimationFrame(done))
    }
    await tick()
    // Open collapsed thoughts and step details that hold the match.
    const lower = needle.toLocaleLowerCase()
    for (const details of rowEl?.querySelectorAll('details') ?? []) {
      if (details.textContent?.toLocaleLowerCase().includes(lower))
        details.open = true
    }
    await tick()
    const inRow = rowEl ? textRanges(rowEl, needle) : []
    const currentRange =
      inRow[Math.min(match.occurrence, inRow.length - 1)] ?? null
    if (typeof Highlight !== 'undefined' && CSS.highlights) {
      const others = textRanges(root, needle).filter(
        (range) =>
          !currentRange ||
          range.startContainer !== currentRange.startContainer ||
          range.startOffset !== currentRange.startOffset,
      )
      CSS.highlights.set('thread-find', new Highlight(...others))
      CSS.highlights.set(
        'thread-find-current',
        new Highlight(...(currentRange ? [currentRange] : [])),
      )
    }
    if (scroll) {
      const target = currentRange?.startContainer.parentElement ?? rowEl
      target?.scrollIntoView({ block: 'center' })
    }
  }

  function step(by: 1 | -1) {
    current = stepMatch(current, matches.length, by)
  }

  function close() {
    open = false
    query = ''
    revealed = ''
    clearHighlights()
    returnFocus?.focus()
    returnFocus = null
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault()
      step(event.shiftKey ? -1 : 1)
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      close()
    }
  }
</script>

{#if open}
  <div
    class="absolute top-2 right-4 z-20 flex items-center gap-1 rounded-lg border border-border bg-popover p-1 pl-2 shadow-md"
    role="search"
    aria-label="Find in conversation"
    data-od-id="thread-find"
  >
    <input
      bind:this={inputEl}
      bind:value={query}
      type="text"
      placeholder="Find in conversation"
      aria-label="Find in conversation"
      autocomplete="off"
      spellcheck="false"
      class="h-7 w-48 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
      onkeydown={onKeydown}
    />
    <span
      class="min-w-16 text-right text-xs text-muted-foreground tabular-nums"
      aria-live="polite"
      data-od-id="thread-find-count">{counter}</span
    >
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label="Previous match"
      title="Previous match (⇧↵)"
      disabled={matches.length < 2}
      onclick={() => step(-1)}
    >
      <ChevronUp class="size-4" aria-hidden="true" />
    </Button>
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label="Next match"
      title="Next match (↵)"
      disabled={matches.length < 2}
      onclick={() => step(1)}
    >
      <ChevronDown class="size-4" aria-hidden="true" />
    </Button>
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label="Close find"
      title="Close (Esc)"
      onclick={close}
    >
      <X class="size-4" aria-hidden="true" />
    </Button>
  </div>
{/if}
