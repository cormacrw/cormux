<script lang="ts">
  import { tick, untrack } from 'svelte'
  import { commands } from '$lib/ipc'
  import { diffComments } from '$lib/changes/diff-comments.svelte'
  import { formatCommentsForAgent } from '$lib/changes/diff-comment-format'
  import { sendThreadMessage } from '$lib/thread/send-message'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { app, workspaceDiff, workspaceUi } from '$lib/state'
  import { formatChangeCounts } from '$lib/workspace/diff-totals'
  import { Button } from '$lib/components/ui/button'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import ChangesFileList from './ChangesFileList.svelte'
  import ChangesTargetPicker from './ChangesTargetPicker.svelte'
  import AgentSpinner from '../thread/AgentSpinner.svelte'
  import ChevronsDownUp from '@lucide/svelte/icons/chevrons-down-up'
  import ChevronsUpDown from '@lucide/svelte/icons/chevrons-up-down'
  import MopSparkles from '@lucide/svelte/icons/mop-sparkles'
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
  // Stale files give way to the splash on a retarget; a refresh keeps what is on screen.
  const pending = $derived(workspaceDiff.pending(workspace.id))
  const loading = $derived(!!pending && (pending.retarget || files.length === 0))

  const allCollapsed = $derived(
    files.length > 0 && files.every((file) => workspaceUi.collapsedDiffPaths[file.path]),
  )

  let scrollEl: HTMLDivElement | undefined = $state()

  // Snapshots rebuild the workspace object, so key the refresh on its id and branch alone.
  const workspaceId = $derived(workspace.id)
  const branch = $derived(workspace.branch)

  $effect(() => {
    void branch
    const id = workspaceId
    untrack(() => {
      void workspaceDiff.fetch(id, async () => {
        const result = await commands.refreshWorkspaceDiff(id)
        return result.status === 'ok'
      })
    })
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

  <div class="relative flex min-h-0 flex-1 flex-col">
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
    {:else if !loading}
      <div class="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
        <MopSparkles class="size-8 text-muted-foreground" aria-hidden="true" />
        <div>
          <h2 class="text-sm font-semibold">Clean diff!</h2>
          <p class="text-sm text-muted-foreground">Go make some changes</p>
        </div>
      </div>
    {/if}
    <!-- Covers the list rather than replacing it, so the diff views stay mounted. -->
    {#if loading}
      <div
        class="changes-splash absolute inset-0 z-20 flex flex-col items-center justify-center gap-3 bg-background px-6 text-center"
        role="status"
      >
        <AgentSpinner />
        <p class="text-sm text-muted-foreground">
          {pending?.retarget
            ? pending.base
              ? `Loading changes since ${pending.base}…`
              : 'Loading uncommitted changes…'
            : 'Loading changes…'}
        </p>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Fast fetches finish before the splash shows, so it never flickers. */
  .changes-splash {
    animation: changes-splash-in 200ms ease-out 150ms both;
  }

  @keyframes changes-splash-in {
    from {
      opacity: 0;
    }
  }
</style>
