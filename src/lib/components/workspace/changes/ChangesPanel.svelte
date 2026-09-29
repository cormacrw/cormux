<script lang="ts">
  import { tick } from 'svelte'
  import { commands } from '$lib/ipc'
  import { diffComments } from '$lib/changes/diff-comments.svelte'
  import { formatCommentsForAgent } from '$lib/changes/diff-comment-format'
  import { sendThreadMessage } from '$lib/thread/send-message'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { app, threadTimeline, workspaceDiff, workspaceUi } from '$lib/state'
  import { formatChangeCounts } from '$lib/workspace/diff-totals'
  import { Button } from '$lib/components/ui/button'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import ChangesFileList from './ChangesFileList.svelte'
  import ChangesTargetPicker from './ChangesTargetPicker.svelte'
  import Clock from '@lucide/svelte/icons/clock'
  import ChevronsDownUp from '@lucide/svelte/icons/chevrons-down-up'
  import ChevronsUpDown from '@lucide/svelte/icons/chevrons-up-down'
  import File from '@lucide/svelte/icons/file'
  import List from '@lucide/svelte/icons/list'
  import Send from '@lucide/svelte/icons/send'

  let {
    workspace,
    thread,
  }: {
    workspace: Workspace
    thread: Thread
  } = $props()

  const files = $derived(workspaceDiff.filesByWorkspace[workspace.id] ?? [])
  const totals = $derived(workspaceDiff.totals(workspace.id))
  const countLabel = $derived(formatChangeCounts(totals))
  const diffBase = $derived(workspaceDiff.base(workspace.id))

  const allCollapsed = $derived(
    files.length > 0 && files.every((file) => workspaceUi.collapsedDiffPaths[file.path]),
  )

  const timelineItems = $derived(threadTimeline.itemsForThread(thread.id, {
    status: thread.status,
    paused: thread.paused,
    role: thread.role,
    workspaceId: thread.workspaceId,
  }, 0))

  const planIdle = $derived(
    files.length === 0 &&
      timelineItems.some((item) => item.kind === 'plan') &&
      thread.status === 'idle',
  )

  let scrollEl: HTMLDivElement | undefined = $state()

  $effect(() => {
    void workspace.branch
    void commands.refreshWorkspaceDiff(workspace.id).catch(() => {})
  })

  // A file link in the conversation expands that file and scrolls to it.
  $effect(() => {
    const path = workspaceUi.revealDiffPath
    if (!path || !files.some((file) => file.path === path)) return
    workspaceUi.revealDiffPath = null
    if (workspaceUi.collapsedDiffPaths[path]) {
      const next = { ...workspaceUi.collapsedDiffPaths }
      delete next[path]
      workspaceUi.collapsedDiffPaths = next
    }
    void tick().then(() => {
      scrollEl
        ?.querySelector(`[data-diff-path="${CSS.escape(path)}"]`)
        ?.scrollIntoView({ block: 'start' })
    })
  })

  function setAllCollapsed(collapsed: boolean) {
    workspaceUi.collapsedDiffPaths = collapsed
      ? Object.fromEntries(files.map((file) => [file.path, true as const]))
      : {}
  }

  function setDiffMode(mode: 'unified' | 'split') {
    const top = scrollEl?.scrollTop ?? 0
    workspaceUi.diffMode = mode
    queueMicrotask(() => {
      if (scrollEl) scrollEl.scrollTop = top
    })
  }

  const comments = $derived(diffComments.list(workspace.id))
  let sending = $state(false)

  async function sendComments() {
    if (!comments.length || sending) return
    sending = true
    const sent = comments
    diffComments.clear(workspace.id)
    if (await sendThreadMessage(thread.id, formatCommentsForAgent(sent))) {
      // Show the conversation so the agent's reply is in view.
      app.threadId = thread.id
      workspaceUi.openTab('thread')
    } else {
      for (const comment of sent) diffComments.add(workspace.id, comment)
    }
    sending = false
  }
</script>

<div
  id="changes-panel"
  role="tabpanel"
  aria-labelledby="thread-tab-changes"
  data-od-id="changes-panel"
  class="flex min-h-0 flex-1 flex-col bg-background"
>
  <div class="flex shrink-0 flex-wrap items-center gap-2 border-b border-border/60 px-3 py-2">
    <ChangesTargetPicker workspaceId={workspace.id} branch={workspace.branch} />
    {#if files.length}
      <ToggleGroup.Root
        type="single"
        value={workspaceUi.diffMode}
        onValueChange={(value) => {
          if (value === 'unified' || value === 'split') setDiffMode(value)
        }}
        variant="outline"
        size="sm"
        aria-label="Diff layout"
      >
        <ToggleGroup.Item value="unified" aria-pressed={workspaceUi.diffMode === 'unified'}>
          Unified
        </ToggleGroup.Item>
        <ToggleGroup.Item value="split" aria-pressed={workspaceUi.diffMode === 'split'}>
          Split
        </ToggleGroup.Item>
      </ToggleGroup.Root>
      {#if countLabel}
        <span class="font-mono text-xs whitespace-nowrap text-muted-foreground">{countLabel}</span>
      {/if}
    {/if}
    <span class="flex-1"></span>
    {#if files.length}
      <Button
        variant="ghost"
        size="sm"
        class="gap-1.5 text-muted-foreground"
        onclick={() => setAllCollapsed(!allCollapsed)}
      >
        {#if allCollapsed}
          <ChevronsUpDown class="size-3.5" aria-hidden="true" /> Expand all
        {:else}
          <ChevronsDownUp class="size-3.5" aria-hidden="true" /> Collapse all
        {/if}
      </Button>
    {/if}
    {#if comments.length}
      <Button
        size="sm"
        class="shrink-0 gap-1.5"
        disabled={sending}
        onclick={() => void sendComments()}
      >
        <Send class="size-3.5" aria-hidden="true" />
        Send {comments.length} {comments.length === 1 ? 'comment' : 'comments'} to {thread.role}
      </Button>
    {/if}
  </div>

  <div class="flex min-h-0 flex-1 flex-col">
    {#if files.length}
      <div
        bind:this={scrollEl}
        class="min-h-0 flex-1 overflow-y-auto"
        tabindex="0"
        role="region"
        aria-label="Proposed changes"
      >
        <ChangesFileList workspaceId={workspace.id} {files} />
      </div>
    {:else}
      <div class="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
        {#if diffBase}
          <File class="size-8 text-muted-foreground" aria-hidden="true" />
          <h2 class="text-sm font-semibold">No commits since {diffBase}</h2>
          <p class="text-sm text-muted-foreground">
            {workspace.branch} has no committed changes since it branched from {diffBase}.
          </p>
        {:else if planIdle}
          <List class="size-8 text-muted-foreground" aria-hidden="true" />
          <h2 class="text-sm font-semibold">Plan ready, nothing edited yet</h2>
          <p class="text-sm text-muted-foreground">
            Review the plan in the thread. Once you approve it, proposed edits stream in here for
            file-by-file review.
          </p>
        {:else}
          <Clock class="size-8 text-muted-foreground" aria-hidden="true" />
          <h2 class="text-sm font-semibold">Waiting for the first edit</h2>
          <p class="text-sm text-muted-foreground">
            The agent is still reading the repository. Diffs appear here as soon as it proposes a
            change.
          </p>
        {/if}
      </div>
    {/if}
  </div>
</div>
