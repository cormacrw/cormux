<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { Button } from '$lib/components/ui/button'
  import { Kbd } from '$lib/components/ui/kbd'
  import CommandIcon from '@lucide/svelte/icons/command'
  import { isHeaderShortcut } from '$lib/keyboard/header-shortcuts'
  import ThreadComposer from '$lib/components/workspace/thread/ThreadComposer.svelte'
  import TimelineMessages from '$lib/components/workspace/thread/TimelineMessages.svelte'
  import {
    app,
    repos,
    scratches,
    shellDialogs,
    threadTimeline,
    threads,
  } from '$lib/state'
  import { scratchStatus, type Scratch } from '$lib/state/scratches.svelte'
  import { engineDisplayName } from '$lib/sidebar/engine'
  import { newestAgentAnnouncement } from '$lib/thread/announce'
  import { createEntryScope, liveReplyId } from '$lib/thread/entering'
  import { buildTimelineRows } from '$lib/thread/timeline-rows'
  import { bindVisibleThreadEvents } from '$lib/workspace/visible-thread-events'
  import ScratchStatusBadge from './ScratchStatusBadge.svelte'

  const scratch = $derived<Scratch | undefined>(
    app.scratchId ? scratches.getById(app.scratchId) : undefined,
  )
  const thread = $derived(
    scratch ? threads.getById(scratch.threadId) : undefined,
  )
  const repo = $derived(scratch ? repos.getById(scratch.repoId) : undefined)
  // A removed repo falls back to its id.
  const repoPath = $derived(repo?.path ?? scratch?.repoId ?? '')
  const status = $derived(scratchStatus(thread))
  const engineName = $derived(engineDisplayName(scratch?.engine ?? 'claude'))

  const items = $derived(
    thread
      ? threadTimeline.itemsForThread(
          thread.id,
          {
            status: thread.status,
            paused: thread.paused,
            role: '',
            workspaceId: thread.workspaceId,
            scratch: true,
          },
          0,
        )
      : [],
  )
  const rows = $derived(buildTimelineRows(items, '', engineName))
  const hasUserMessage = $derived(items.some((item) => item.kind === 'user'))

  const liveTitle = $derived(
    thread?.paused ? 'Paused by you' : 'Reading the repo…',
  )
  const liveSubtitle = $derived(
    thread?.paused
      ? 'Resume to let the agent continue.'
      : ((thread && threadTimeline.currentToolByThread[thread.id]) ??
          'Finding the files this question names'),
  )

  let headingEl = $state<HTMLHeadingElement | null>(null)
  let scrollEl = $state<HTMLElement | null>(null)
  let contentEl = $state<HTMLElement | null>(null)
  // Off once the reader scrolls up; back on when they return to the end.
  let pinnedToBottom = true
  let focusComposer = $state<(() => void) | null>(null)
  let announceText = $state('')
  let nowMs = $state(Date.now())
  let prevItemCount = 0
  let prevThreadId: string | null = null
  let scrollToEnd = $state(true)

  // Opening a scratch: title first, then the composer for a blank start.
  $effect(() => {
    if (app.focusTarget !== 'scratch') return
    void app.focusGeneration
    void tick().then(() => {
      headingEl?.focus()
      if (app.scratchComposerFocus) {
        app.scratchComposerFocus = false
        focusComposer?.()
      }
    })
  })

  // Snapshot what the scratch already shows when it opens; only rows after that animate.
  // The thread object is replaced on every status change, so depend on the id alone.
  const threadId = $derived(thread?.id)
  const entries = $derived.by(() => {
    void threadId
    return untrack(() => createEntryScope(rows))
  })

  // Only items added while this scratch is on screen get announced.
  $effect(() => {
    const count = items.length
    const id = thread?.id ?? null
    if (id !== prevThreadId) {
      prevThreadId = id
      prevItemCount = count
      scrollToEnd = true
      return
    }
    if (count > prevItemCount) {
      const text = newestAgentAnnouncement(items, engineName)
      if (text) announceText = text
      scrollToEnd = true
    }
    prevItemCount = count
  })

  $effect(() => {
    if (!scrollToEnd || !scrollEl) return
    const element = scrollEl
    queueMicrotask(() => {
      element.scrollTop = element.scrollHeight
      pinnedToBottom = true
      scrollToEnd = false
    })
  })

  // A reply typing out grows its row without adding one, so follow the content's height too.
  $effect(() => {
    const scroller = scrollEl
    const content = contentEl
    if (!scroller || !content) return
    const observer = new ResizeObserver(() => {
      if (pinnedToBottom) scroller.scrollTop = scroller.scrollHeight
    })
    observer.observe(content)
    return () => observer.disconnect()
  })

  function onThreadScroll() {
    if (!scrollEl) return
    const gap =
      scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight
    pinnedToBottom = gap < 80
  }

  $effect(() => {
    const id = thread?.id
    if (!id) return
    return bindVisibleThreadEvents(id, (event) => {
      threadTimeline.applyEvent(id, event)
    })
  })

  onMount(() => {
    const timer = setInterval(() => {
      nowMs = Date.now()
    }, 60_000)
    return () => clearInterval(timer)
  })

  function startHeaderDrag(event: MouseEvent) {
    if (event.button !== 0) return
    const target = event.target
    if (!(target instanceof Element)) return
    if (target.closest('button, a, input, textarea')) return
    void getCurrentWindow().startDragging()
  }

  function onKeydown(event: KeyboardEvent) {
    if (!scratch || !isHeaderShortcut(event, 'e')) return
    event.preventDefault()
    shellDialogs.openEndScratch(scratch.id, false)
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if scratch && thread}
  <section
    class="view flex min-h-0 flex-1 flex-col"
    id="view-session"
    aria-label="Scratch"
    data-od-id="session-view"
  >
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <header
      class="ws-top flex items-center justify-between gap-4 border-b border-border/60 px-5 py-3"
      data-tauri-drag-region
      data-od-id="session-header"
      onmousedown={startHeaderDrag}
    >
      <div class="flex min-w-0 flex-1 flex-col gap-0.5">
        <h1
          bind:this={headingEl}
          id="sess-heading"
          tabindex="-1"
          class="text-[15px] font-medium tracking-tight outline-none [text-wrap:pretty]"
        >
          {scratch.title}
        </h1>
        <p
          class="truncate font-mono text-xs text-muted-foreground"
          title={repoPath}
        >
          {repoPath}
        </p>
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <ScratchStatusBadge {status} />
        <Button
          variant="secondary"
          size="xl"
          data-action="end-scratch"
          data-od-id="session-end"
          aria-keyshortcuts="Meta+E"
          onclick={() => shellDialogs.openEndScratch(scratch.id, false)}
        >
          End scratch
          <Kbd class="gap-0.5" aria-hidden="true"><CommandIcon />E</Kbd>
        </Button>
      </div>
    </header>

    <div
      bind:this={scrollEl}
      class="min-h-0 flex-1 overflow-y-auto"
      data-od-id="session-thread"
      onscroll={onThreadScroll}
    >
      <div
        bind:this={contentEl}
        class="mx-auto w-full max-w-[760px] px-4 pb-4 pt-6 select-text"
      >
        <ol class="space-y-2" aria-label="Conversation">
          <TimelineMessages
            {rows}
            engine={engineName}
            workspaceId={scratch.id}
            findings={[]}
            {nowMs}
            {liveTitle}
            {liveSubtitle}
            paused={thread.paused}
            {entries}
            liveReplyId={liveReplyId(items)}
            onOpenFindings={() => {}}
            onFocusComposer={() => focusComposer?.()}
          />
        </ol>
      </div>
    </div>

    <div id="sess-announce" class="sr-only" aria-live="polite">
      {announceText}
    </div>

    <div data-od-id="session-composer">
      <ThreadComposer
        {thread}
        bind:focusComposer
        inputLabel="Message this scratch"
        placeholder={hasUserMessage
          ? 'Ask a follow-up…'
          : 'Write the first message…'}
        sendLabel="Send"
        onSent={() => {
          scrollToEnd = true
        }}
      />
    </div>
  </section>
{/if}
