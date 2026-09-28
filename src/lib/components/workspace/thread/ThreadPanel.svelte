<script lang="ts">
  import { onMount } from 'svelte'
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
  let announceText = $state('')
  let nowMs = $state(Date.now())
  let isNewIds = $state<Record<string, true>>({})
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

  const timeBaseMs = $derived(workspace.createdAtMs ?? Date.now() - 60_000)

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

  $effect(() => {
    $virtualizer.setOptions({
      count: rows.length,
      getScrollElement: () => scrollEl ?? null,
    })
  })

  $effect(() => {
    if (thread.id !== prevThreadId) {
      prevThreadId = thread.id
      scrollToEndNext = true
      prevItemCount = 0
      isNewIds = {}
    }
  })

  $effect(() => {
    const count = items.length
    if (count > prevItemCount && prevItemCount > 0) {
      const fresh: Record<string, true> = {}
      for (let i = prevItemCount; i < count; i += 1) {
        const item = items[i]
        if (item && item.kind !== 'live') fresh[item.id] = true
      }
      isNewIds = fresh
      const text = newestAgentAnnouncement(items, thread.role)
      if (text) announceText = text
      scrollToEndNext = true
    }
    prevItemCount = count
  })

  $effect(() => {
    void app.threadId
    void workspaceUi.activeTab
    scrollToEndNext = true
  })

  $effect(() => {
    if (!scrollToEndNext || !scrollEl) return
    queueMicrotask(() => {
      scrollEl!.scrollTop = scrollEl!.scrollHeight
      scrollToEndNext = false
    })
  })

  $effect(() => {
    if (!thread.id) return
    return bindVisibleThreadEvents(thread.id, (event) => {
      threadTimeline.applyEvent(thread.id, event)
    })
  })

  onMount(() => {
    const timer = setInterval(() => {
      nowMs = Date.now()
    }, 60_000)
    return () => clearInterval(timer)
  })

  function openFindingsTab() {
    workspaceUi.findingsFocusPending = true
    workspaceUi.openTab('findings')
  }
</script>

<section
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
    <div class="mx-auto w-full max-w-[760px] px-1 pb-4 pt-1">
      <ThreadIntro {thread} {workspace} {otherThreadCount} {nowMs} />

      <ol class="mt-4 space-y-2" aria-label="Conversation" id="timeline">
        {#if useVirtual}
          <div
            style={`height: ${$virtualizer.getTotalSize()}px; position: relative;`}
          >
            {#each $virtualizer.getVirtualItems() as virtualRow (virtualRow.key)}
              {@const row = rows[virtualRow.index]}
              <div
                style={`position: absolute; top: 0; left: 0; width: 100%; transform: translateY(${virtualRow.start}px);`}
              >
                {#if row}
                  <TimelineMessages
                    rows={[row]}
                    engine={engineDisplayName(thread.engine)}
                    workspaceId={workspace.id}
                    findings={workspaceFindings}
                    {nowMs}
                    {timeBaseMs}
                    liveTitle={liveRowTitle}
                    liveSubtitle={liveRowSubtitle}
                    paused={thread.paused}
                    {isNewIds}
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
            {timeBaseMs}
            liveTitle={liveRowTitle}
            liveSubtitle={liveRowSubtitle}
            paused={thread.paused}
            {isNewIds}
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
    bind:focusComposer
    onSent={() => {
      scrollToEndNext = true
    }}
  />
</section>
