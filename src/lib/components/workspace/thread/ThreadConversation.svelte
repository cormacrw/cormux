<!--
  A thread's conversation: the scrolling timeline with find, follows the newest message while
  you're at the end, pops in what's new and announces it. The thread panel and a scratch both
  draw their thread with it.
-->
<script lang="ts" module>
  import { SvelteSet } from 'svelte/reactivity'

  // A registry nothing renders; the reactive state is the sets inside it.
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  const foldsByThread = new Map<string, SvelteSet<string>>()

  function foldsFor(threadId: string) {
    let folds = foldsByThread.get(threadId)
    if (!folds) {
      folds = new SvelteSet<string>()
      foldsByThread.set(threadId, folds)
    }
    return folds
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte'
  import { onDestroy, onMount, tick, untrack } from 'svelte'
  import type { ClassValue } from 'svelte/elements'
  import { createVirtualizer } from '@tanstack/svelte-virtual'
  import { Button } from '$lib/components/ui/button'
  import { enterConversation } from '$lib/thread/conversation-motion'
  import { threadTimeline } from '$lib/thread/timeline-store.svelte'
  import { newestAgentAnnouncement } from '$lib/thread/announce'
  import { createEntryScope } from '$lib/thread/entering'
  import { buildTimelineRows } from '$lib/thread/timeline-rows'
  import type { ConversationSource, RowContext } from './rows/context'
  import LiveRow from './rows/LiveRow.svelte'
  import ThreadFind from './ThreadFind.svelte'
  import TimelineMessages from './TimelineMessages.svelte'

  let {
    threadId,
    role,
    engine,
    source,
    shown = true,
    scratch = false,
    liveInline = false,
    odId,
    contentClass,
    padBottom,
    hero,
    intro,
    aside,
  }: {
    threadId: string
    /** Who the agent is in this thread, named over its turns; empty to name it by its engine. */
    role: string
    engine: string
    /** Where its edits, media and links resolve. */
    source: ConversationSource
    /** On screen rather than under a slide-out; coming back lands on the newest message. */
    shown?: boolean
    scratch?: boolean
    /** Draw what the agent is doing as the last row, for a thread with no prompt bar to float it over. */
    liveInline?: boolean
    odId?: string
    contentClass?: ClassValue
    padBottom?: string
    /** Shown instead of the timeline while the conversation is empty. */
    hero?: Snippet<[nowMs: number]>
    /** Above the timeline. */
    intro?: Snippet<[nowMs: number]>
    /** Beside the scrolling conversation. */
    aside?: Snippet
  } = $props()

  const view = $derived(threadTimeline.threadView(threadId))
  const conversationId = $derived(view.conversations.current.id)
  const loaded = $derived(view.loaded)
  const generation = $derived(view.generation)
  const showHero = $derived(Boolean(hero) && view.loaded && !view.items.length)
  const rows = $derived(buildTimelineRows(view.items, role, engine))
  const useVirtual = $derived(rows.length > 150)

  let scrollEl: HTMLElement | undefined = $state()
  let contentEl: HTMLElement | undefined = $state()
  let announceText = $state('')
  let nowMs = $state(Date.now())

  $effect(() => threadTimeline.watch(threadId))

  onMount(() => {
    const timer = setInterval(() => {
      nowMs = Date.now()
    }, 60_000)
    return () => clearInterval(timer)
  })

  // Snapshot what the thread already shows when it opens, or switches to another
  // conversation; only rows after that animate. Rows read from the store, older pages
  // included, were already there.
  const entries = $derived.by(() => {
    void threadId
    void conversationId
    void loaded
    void generation
    return untrack(() => createEntryScope(rows))
  })

  // Made here rather than where a row first reads it: a set made inside the reading row's own
  // derived isn't tracked by it, so opening a fold wouldn't redraw.
  const openFolds = $derived(foldsFor(threadId))

  const ctx: RowContext = {
    get entries() {
      return entries
    },
    get nowMs() {
      return nowMs
    },
    get activeIds() {
      return view.activeIds
    },
    get openFolds() {
      return openFolds
    },
    get workdir() {
      return source.workdir
    },
    diffFile: (path) => source.diffFile(path),
    findPr: (repoFullName, num) => source.findPr(repoFullName, num),
    get revealFile() {
      return source.revealFile
    },
    get scratch() {
      return scratch
    },
  }

  // Switching conversation within a thread (new session, resume) rises the new one in.
  let shownConversation = ''
  $effect(() => {
    const key = `${threadId}:${conversationId}`
    const previous = shownConversation
    shownConversation = key
    if (previous && previous !== key && previous.startsWith(`${threadId}:`)) {
      void tick().then(enterConversation)
    }
  })

  let loadingOlder = $state(false)

  async function showEarlier() {
    const scroller = scrollEl
    if (!scroller || loadingOlder) return
    loadingOlder = true
    pinnedToBottom = false
    const height = scroller.scrollHeight
    const top = scroller.scrollTop
    await threadTimeline.loadOlder(threadId)
    await tick()
    // Stays on the same message as the rows above it appear.
    scroller.scrollTop = top + scroller.scrollHeight - height
    loadingOlder = false
  }

  // Whether the reader is at the bottom; streamed replies grow a row without adding one,
  // so follow height changes while pinned instead of only on new rows.
  let pinnedToBottom = true
  // The last scrollTop we saw or set, and when the reader last touched the thread. Only the
  // reader moving up unpins: the virtual list also moves scrollTop up when rows re-measure.
  let lastScrollTop = 0
  let readerInputAt = 0
  const READER_INPUT_MS = 500

  /** Back to the newest message, and following it again. */
  export function followEnd() {
    pinnedToBottom = true
    queueMicrotask(() => {
      if (scrollEl) scrollTo(scrollEl)
      keepFollowing()
    })
  }

  let prevThreadId: string | null = null
  let prevItemCount = 0
  $effect(() => {
    const count = view.items.length
    untrack(() => {
      if (threadId !== prevThreadId) {
        prevThreadId = threadId
        prevItemCount = count
        followEnd()
        return
      }
      if (count > prevItemCount && prevItemCount > 0) {
        const text = newestAgentAnnouncement(view.items, role || engine)
        if (text) announceText = text
        if (stillFollowing()) followEnd()
      }
      prevItemCount = count
    })
  })

  // Every chunk changes the items; resize notifications alone can be dropped.
  $effect(() => {
    void view.items
    untrack(() => {
      if (pinnedToBottom) keepFollowing()
    })
  })

  // Coming back to the conversation lands on the newest message.
  $effect(() => {
    if (shown) untrack(followEnd)
  })

  const virtualizer = createVirtualizer<HTMLElement, Element>({
    count: 0,
    getScrollElement: () => scrollEl ?? null,
    estimateSize: () => 96,
    overscan: 8,
  })

  // When a row above the fold re-measures, the list shifts scrollTop from the offset it last
  // saw, which lags our own writes, so following the end got yanked back up for a frame.
  // While pinned the follow loop owns the scroll; scrolled up, the list keeps the reader's place.
  Object.defineProperty(
    $virtualizer,
    'shouldAdjustScrollPositionOnItemSizeChange',
    { get: () => (pinnedToBottom ? () => false : undefined) },
  )

  $effect(() => {
    const count = rows.length
    const element = scrollEl
    untrack(() => {
      $virtualizer.setOptions({
        count,
        getScrollElement: () => element ?? null,
      })
    })
  })

  $effect(() => {
    const scroller = scrollEl
    const content = contentEl
    if (!scroller || !content) return
    const onScroll = () => {
      pinnedToBottom = !readerScrolledUp(scroller)
      lastScrollTop = scroller.scrollTop
    }
    const onReaderInput = () => {
      readerInputAt = performance.now()
    }
    // WebKit scrolls off the main thread, so the follow loop can land before scrollTop
    // moves; an upward wheel unpins straight away instead of waiting for the scroll.
    const onWheel = (event: WheelEvent) => {
      onReaderInput()
      if (event.deltaY < 0 && scroller.scrollTop > 0) pinnedToBottom = false
    }
    const inputs = ['touchmove', 'keydown', 'pointerdown'] as const
    scroller.addEventListener('scroll', onScroll, { passive: true })
    scroller.addEventListener('wheel', onWheel, { passive: true })
    for (const name of inputs) {
      scroller.addEventListener(name, onReaderInput, { passive: true })
    }
    followObserver.observe(content)
    return () => {
      scroller.removeEventListener('scroll', onScroll)
      scroller.removeEventListener('wheel', onWheel)
      for (const name of inputs)
        scroller.removeEventListener(name, onReaderInput)
      followObserver.unobserve(content)
    }
  })

  // Follows the end as content grows. Virtual rows are positioned absolutely, so a row that
  // grows (a reply typing out) can outrun the list's measured height; they're observed too.
  // Observers fire after layout but before paint, so catching up here means the frame never
  // shows the old offset; waiting for the next frame painted a jump on every re-measure.
  const followObserver = new ResizeObserver(() => {
    if (scrollEl && stillFollowing()) scrollTo(scrollEl)
    keepFollowing()
  })

  // WebKit drops resize notifications when rows re-measure in the same frame, so once
  // something grows, re-check every frame until the end has held still for a moment.
  const SETTLE_FRAMES = 8
  let followFrame = 0
  let stillFrames = 0

  function keepFollowing() {
    stillFrames = 0
    if (!followFrame) followFrame = requestAnimationFrame(followStep)
  }

  function followStep() {
    followFrame = 0
    const scroller = scrollEl
    if (!scroller || !stillFollowing()) return
    const behind =
      scroller.scrollHeight - scroller.clientHeight - scroller.scrollTop
    if (behind > 1) {
      scrollTo(scroller)
      stillFrames = 0
    } else {
      stillFrames += 1
    }
    if (stillFrames < SETTLE_FRAMES)
      followFrame = requestAnimationFrame(followStep)
  }

  onDestroy(() => {
    followObserver.disconnect()
    cancelAnimationFrame(followFrame)
  })

  function scrollTo(scroller: HTMLElement) {
    scroller.scrollTop = scroller.scrollHeight
    lastScrollTop = scroller.scrollTop
  }

  // Any upward move by the reader unpins, however small: trackpads scroll a few pixels per
  // event, and following would snap each one back before it added up to a real distance.
  function readerScrolledUp(scroller: HTMLElement) {
    const byReader = performance.now() - readerInputAt < READER_INPUT_MS
    if (byReader && scroller.scrollTop < lastScrollTop - 1) return true
    const gap =
      scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight
    if (gap < 80) return false
    return !pinnedToBottom
  }

  // Checked before the scroll event lands, so a row arriving right after the reader
  // scrolls up doesn't pull them back down.
  function stillFollowing() {
    return pinnedToBottom && (!scrollEl || !readerScrolledUp(scrollEl))
  }

  function measureRow(node: HTMLElement) {
    $virtualizer.measureElement(node)
    followObserver.observe(node)
    return { destroy: () => followObserver.unobserve(node) }
  }
</script>

<div class="relative flex min-h-0 flex-1 flex-col">
  <ThreadFind
    {rows}
    {contentEl}
    scrollToRow={useVirtual
      ? (index) => $virtualizer.scrollToIndex(index, { align: 'center' })
      : undefined}
    onScrollAway={() => {
      pinnedToBottom = false
    }}
  />
  {@render aside?.()}
  <div
    id="thread-scroll"
    bind:this={scrollEl}
    class="@container min-h-0 flex-1 overflow-y-auto"
    data-od-id={odId}
  >
    <div
      bind:this={contentEl}
      class={['mx-auto w-full max-w-[760px] px-4 select-text', contentClass]}
      style:padding-bottom={padBottom}
    >
      {#if !view.loaded}
        <!-- Reads in a moment; a greeting here would flash before the conversation. -->
      {:else if showHero}
        {@render hero?.(nowMs)}
      {:else if intro}
        <div data-conversation-part>
          {@render intro(nowMs)}
        </div>
      {/if}

      {#if view.hasOlder && !showHero}
        <div class="mt-3 flex justify-center">
          <Button
            variant="ghost"
            size="sm"
            data-od-id="thread-show-earlier"
            disabled={loadingOlder}
            onclick={showEarlier}
          >
            Show earlier messages
          </Button>
        </div>
      {/if}

      <ol
        class={['space-y-2', intro && 'mt-4', showHero && 'hidden']}
        aria-label="Conversation"
        id="timeline"
        data-conversation-part
      >
        {#if useVirtual}
          <div
            style={`height: ${$virtualizer.getTotalSize()}px; position: relative;`}
          >
            {#each $virtualizer.getVirtualItems() as virtualRow (virtualRow.key)}
              {@const row = rows[virtualRow.index]}
              <div
                data-index={virtualRow.index}
                use:measureRow
                class="pb-2"
                style={`position: absolute; top: 0; left: 0; width: 100%; transform: translateY(${virtualRow.start}px);`}
              >
                {#if row}
                  <TimelineMessages rows={[row]} {ctx} />
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <TimelineMessages {rows} {ctx} />
        {/if}
        {#if liveInline && view.live}
          <LiveRow live={view.live} />
        {/if}
      </ol>
    </div>
  </div>
</div>

<div id="thread-announce" class="sr-only" aria-live="polite">
  {announceText}
</div>
