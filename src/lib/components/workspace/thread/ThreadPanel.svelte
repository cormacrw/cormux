<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte'
  import {
    app,
    findings,
    threadTimeline,
    threads,
    workspaceUi,
  } from '$lib/state'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { engineDisplayName } from '$lib/sidebar/engine'
  import { newestAgentAnnouncement } from '$lib/thread/announce'
  import { createEntryScope, liveReplyId } from '$lib/thread/entering'
  import { liveSubtitle, liveTitle } from '$lib/thread/map-events-to-timeline'
  import { buildTimelineRows } from '$lib/thread/timeline-rows'
  import { bindVisibleThreadEvents } from '$lib/workspace/visible-thread-events'
  import { createVirtualizer } from '@tanstack/svelte-virtual'
  import ThreadComposer from './ThreadComposer.svelte'
  import ThreadIntro from './ThreadIntro.svelte'
  import TimelineMessages from './TimelineMessages.svelte'

  let {
    workspace,
    thread,
    panelLabelId,
  }: {
    workspace: Workspace
    thread: Thread
    panelLabelId: string | undefined
  } = $props()

  let scrollEl: HTMLElement | undefined = $state()
  let contentEl: HTMLElement | undefined = $state()
  // Whether the reader is at the bottom; streamed replies grow a row without adding one,
  // so follow height changes while pinned instead of only on new rows.
  let pinnedToBottom = true
  // The last scrollTop we saw or set, and when the reader last touched the thread. Only the
  // reader moving up unpins: the virtual list also moves scrollTop up when rows re-measure.
  let lastScrollTop = 0
  let readerInputAt = 0
  const READER_INPUT_MS = 500
  let announceText = $state('')
  let nowMs = $state(Date.now())
  let prevItemCount = $state(0)
  let prevThreadId = $state<string | null>(null)
  let scrollToEndNext = $state(true)
  let focusComposer = $state<(() => void) | null>(null)

  const wsThreads = $derived(threads.forWorkspace(workspace.id))
  const otherThreadCount = $derived(
    Math.max(0, wsThreads.filter((row) => row.id !== thread.id).length),
  )
  const workspaceFindings = $derived(findings.forWorkspace(workspace.id))

  const items = $derived(
    threadTimeline.itemsForThread(
      thread.id,
      {
        status: thread.status,
        paused: thread.paused,
        role: thread.role,
        workspaceId: thread.workspaceId,
      },
      workspaceFindings.length,
    ),
  )

  const rows = $derived(
    buildTimelineRows(items, thread.role, engineDisplayName(thread.engine)),
  )

  const useVirtual = $derived(rows.length > 150)
  const typingReplyId = $derived(liveReplyId(items))

  // Snapshot what the thread already shows when it opens; only rows after that animate.
  // The thread object is replaced on every status change, so depend on the id alone.
  const threadId = $derived(thread.id)
  const entries = $derived.by(() => {
    void threadId
    return untrack(() => createEntryScope(rows))
  })

  const liveRowTitle = $derived(
    liveTitle({
      status: thread.status,
      paused: thread.paused,
      activity: thread.activity,
    }),
  )

  const liveRowSubtitle = $derived(
    liveSubtitle({
      status: thread.status,
      paused: thread.paused,
      isLead: thread.role === 'Lead',
      currentToolTitle: threadTimeline.currentToolByThread[thread.id] ?? null,
      activity: thread.activity,
    }),
  )

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
    if (thread.id !== prevThreadId) {
      prevThreadId = thread.id
      pinnedToBottom = true
      scrollToEndNext = true
      prevItemCount = 0
    }
  })

  // Every chunk changes the items; resize notifications alone can be dropped.
  $effect(() => {
    void items
    untrack(() => {
      if (pinnedToBottom) keepFollowing()
    })
  })

  $effect(() => {
    const count = items.length
    if (count > prevItemCount && prevItemCount > 0) {
      const text = newestAgentAnnouncement(items, thread.role)
      if (text) announceText = text
      if (stillFollowing()) scrollToEndNext = true
    }
    prevItemCount = count
  })

  $effect(() => {
    void app.threadId
    void workspaceUi.activeTab
    pinnedToBottom = true
    scrollToEndNext = true
  })

  $effect(() => {
    if (!scrollToEndNext || !scrollEl) return
    queueMicrotask(() => {
      followToEnd(scrollEl!)
      scrollToEndNext = false
      keepFollowing()
    })
  })

  $effect(() => {
    if (!thread.id) return
    return bindVisibleThreadEvents(thread.id, (event) => {
      threadTimeline.applyEvent(thread.id, event)
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
    if (scrollEl && stillFollowing()) followToEnd(scrollEl)
    keepFollowing()
  })

  // WebKit drops resize notifications when rows re-measure in the same frame, so once
  // something grows, re-check every frame until the end has held still for a moment.
  const SETTLE_FRAMES = 30
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
      followToEnd(scroller)
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

  onMount(() => {
    const timer = setInterval(() => {
      nowMs = Date.now()
    }, 60_000)
    return () => clearInterval(timer)
  })

  function followToEnd(scroller: HTMLElement) {
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

  // Measure real row heights (and re-measure on resize) instead of the 96px estimate.
  function measureRow(node: HTMLElement) {
    $virtualizer.measureElement(node)
    followObserver.observe(node)
    return { destroy: () => followObserver.unobserve(node) }
  }

  function openFindingsTab() {
    workspaceUi.findingsFocusPending = true
    workspaceUi.openTab('findings')
  }
</script>

<div
  id="thread-panel"
  role="tabpanel"
  aria-labelledby={panelLabelId}
  data-od-id="thread-panel"
  class="flex min-h-0 flex-1 flex-col"
>
  <div
    id="thread-scroll"
    bind:this={scrollEl}
    class="min-h-0 flex-1 overflow-y-auto"
  >
    <div
      bind:this={contentEl}
      class="mx-auto w-full max-w-[760px] px-4 pb-4 pt-3"
    >
      <ThreadIntro {thread} {workspace} {otherThreadCount} {nowMs} />

      <ol class="mt-4 space-y-2" aria-label="Conversation" id="timeline">
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
                  <TimelineMessages
                    rows={[row]}
                    engine={engineDisplayName(thread.engine)}
                    workspaceId={workspace.id}
                    findings={workspaceFindings}
                    {nowMs}
                    liveTitle={liveRowTitle}
                    liveSubtitle={liveRowSubtitle}
                    paused={thread.paused}
                    {entries}
                    liveReplyId={typingReplyId}
                    onOpenFindings={openFindingsTab}
                    onFocusComposer={() => focusComposer?.()}
                  />
                {/if}
              </div>
            {/each}
          </div>
        {:else}
          <TimelineMessages
            {rows}
            engine={engineDisplayName(thread.engine)}
            workspaceId={workspace.id}
            findings={workspaceFindings}
            {nowMs}
            liveTitle={liveRowTitle}
            liveSubtitle={liveRowSubtitle}
            paused={thread.paused}
            {entries}
            liveReplyId={typingReplyId}
            onOpenFindings={openFindingsTab}
            onFocusComposer={() => focusComposer?.()}
          />
        {/if}
      </ol>
    </div>
  </div>

  <div id="thread-announce" class="sr-only" aria-live="polite">
    {announceText}
  </div>

  <ThreadComposer
    {thread}
    newSession
    bind:focusComposer
    onSent={() => {
      pinnedToBottom = true
      scrollToEndNext = true
    }}
  />
</div>
