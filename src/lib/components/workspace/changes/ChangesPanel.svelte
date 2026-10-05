<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import { afterPaint } from '$lib/changes/mount-queue'
  import { commands } from '$lib/ipc'
  import {
    commentBranch,
    diffComments,
  } from '$lib/changes/diff-comments.svelte'
  import { formatCommentsForAgent } from '$lib/changes/diff-comment-format'
  import { sendThreadMessage } from '$lib/thread/send-message'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { app, workspaceDiff, workspaceUi } from '$lib/state'
  import { formatChangeCounts } from '$lib/workspace/diff-totals'
  import { isDiffCollapsed } from '$lib/changes/file-status'
  import { revealDiffLine } from '$lib/changes/reveal-line'
  import { Button } from '$lib/components/ui/button'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { diffTargetLabel } from '$lib/stack/stack'
  import ChangesFileList from './ChangesFileList.svelte'
  import ChangesSkeleton from './ChangesSkeleton.svelte'
  import StackRail from './StackRail.svelte'
  import ChevronsDownUp from '@lucide/svelte/icons/chevrons-down-up'
  import ChevronsUpDown from '@lucide/svelte/icons/chevrons-up-down'
  import GitCompare from '@lucide/svelte/icons/git-compare'
  import MopSparkles from '@lucide/svelte/icons/mop-sparkles'
  import Send from '@lucide/svelte/icons/send'
  import Trash2 from '@lucide/svelte/icons/trash-2'

  let {
    workspace,
    thread,
    repoId,
    provisioning,
  }: {
    workspace: Workspace
    thread: Thread
    repoId: string
    provisioning: boolean
  } = $props()

  const files = $derived(workspaceDiff.filesByWorkspace[workspace.id] ?? [])
  const totals = $derived(workspaceDiff.totals(workspace.id))
  const countLabel = $derived(formatChangeCounts(totals))
  // Stale files give way to the splash on a retarget; a refresh keeps what is on screen.
  const pending = $derived(workspaceDiff.pending(workspace.id))
  const loading = $derived(
    !!pending && (pending.retarget || files.length === 0),
  )
  // While a retarget loads, the label shows where it is headed.
  const shownTarget = $derived(
    pending?.retarget ? pending.target : workspaceDiff.target(workspace.id),
  )

  const allCollapsed = $derived(
    files.length > 0 &&
      files.every((file) =>
        isDiffCollapsed(workspaceUi.collapsedDiffPaths, file.path),
      ),
  )

  let scrollEl: HTMLDivElement | undefined = $state()

  // The tab switches the moment it's clicked: the skeleton paints first and the file
  // list, which can be long, mounts in the frame after.
  let listReady = $state(false)
  onMount(() => afterPaint(() => (listReady = true)))

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

  // A file link (conversation, findings) expands that file and scrolls to it, or to its line.
  $effect(() => {
    const path = workspaceUi.revealDiffPath
    if (!path || !files.some((file) => file.path === path)) return
    const line = workspaceUi.revealDiffLine
    workspaceUi.revealDiffPath = null
    workspaceUi.revealDiffLine = null
    if (isDiffCollapsed(workspaceUi.collapsedDiffPaths, path)) {
      workspaceUi.collapsedDiffPaths = {
        ...workspaceUi.collapsedDiffPaths,
        [path]: false,
      }
    }
    void tick().then(() => {
      const fileEl = scrollEl?.querySelector(
        `[data-diff-path="${CSS.escape(path)}"]`,
      )
      fileEl?.scrollIntoView({ block: 'start' })
      if (fileEl && line != null) revealDiffLine(fileEl, line)
    })
  })

  function setAllCollapsed(collapsed: boolean) {
    workspaceUi.collapsedDiffPaths = Object.fromEntries(
      files.map((file) => [file.path, collapsed]),
    )
  }

  function setDiffMode(mode: 'unified' | 'split') {
    const top = scrollEl?.scrollTop ?? 0
    workspaceUi.diffMode = mode
    queueMicrotask(() => {
      if (scrollEl) scrollEl.scrollTop = top
    })
  }

  // Comments belong to the level on screen, not to whatever is mid-load.
  const commentScope = $derived(
    commentBranch(workspaceDiff.target(workspace.id), workspace.branch),
  )
  const comments = $derived(diffComments.forBranch(workspace.id, commentScope))
  let sending = $state(false)
  // The agent works on the checked-out branch, so comments on another level wait until it's checked out.
  const offBranch = $derived(
    commentScope !== null && commentScope !== workspace.branch,
  )

  async function sendComments() {
    if (!comments.length || sending || offBranch) return
    sending = true
    const sent = comments
    diffComments.clear(workspace.id, commentScope)
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
  class="flex min-h-0 flex-1"
>
  <StackRail {workspace} {repoId} {provisioning} />
  <div class="felt-sm m-3 ml-1.5 flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
    <div
      class="flex shrink-0 flex-wrap items-center gap-2 px-4 py-3"
    >
      <span
        class="flex h-8 min-w-0 items-center gap-1.5 font-display text-lg font-extrabold"
        data-od-id="changes-target"
        title={shownTarget
          ? `Committed changes on ${shownTarget.head} since it left ${shownTarget.base}`
          : 'Uncommitted changes vs HEAD'}
      >
        <GitCompare
          class="size-3.5 shrink-0 text-muted-foreground"
          aria-hidden="true"
        />
        <span class="truncate">{diffTargetLabel(shownTarget)}</span>
      </span>
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
          <ToggleGroup.Item
            value="unified"
            aria-pressed={workspaceUi.diffMode === 'unified'}
          >
            Unified
          </ToggleGroup.Item>
          <ToggleGroup.Item
            value="split"
            aria-pressed={workspaceUi.diffMode === 'split'}
          >
            Split
          </ToggleGroup.Item>
        </ToggleGroup.Root>
        {#if countLabel}
          <span
            class="font-mono text-xs whitespace-nowrap text-muted-foreground"
            >{countLabel}</span
          >
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
          variant="ghost"
          size="icon-sm"
          class="shrink-0 text-muted-foreground hover:text-destructive"
          disabled={sending}
          title="Clear every unsent comment on this level"
          aria-label="Clear comments"
          onclick={() => diffComments.clear(workspace.id, commentScope)}
        >
          <Trash2 class="size-3.5" aria-hidden="true" />
        </Button>
        <!-- The wrapper carries the tooltip, since a disabled button gets no hover. -->
        <span
          class="shrink-0"
          title={offBranch
            ? `These comments are on ${commentScope}. Check it out to send them to ${thread.role}.`
            : undefined}
        >
          <Button
            size="sm"
            class="gap-1.5"
            disabled={sending || offBranch}
            onclick={() => void sendComments()}
          >
            <Send class="size-3.5" aria-hidden="true" />
            Send {comments.length}
            {comments.length === 1 ? 'comment' : 'comments'} to {thread.role}
          </Button>
        </span>
      {/if}
    </div>

    <div class="relative flex min-h-0 flex-1 flex-col">
      {#if !listReady || (loading && !files.length)}
        <ChangesSkeleton />
      {:else if files.length}
        <!-- Focusable so the diff can be scrolled from the keyboard. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          bind:this={scrollEl}
          class="min-h-0 flex-1 overflow-y-auto [contain:strict]"
          tabindex="0"
          role="region"
          aria-label="Proposed changes"
        >
          <ChangesFileList
            workspaceId={workspace.id}
            branch={commentScope}
            {files}
          />
        </div>
      {:else if !loading}
        <div
          class="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center"
        >
          <MopSparkles
            class="size-8 text-muted-foreground"
            aria-hidden="true"
          />
          <div>
            <h2 class="text-sm font-semibold">Clean diff!</h2>
            <p class="text-sm text-muted-foreground">
              {shownTarget
                ? `No commits on ${shownTarget.head} since ${shownTarget.base}`
                : 'Go make some changes'}
            </p>
          </div>
        </div>
      {/if}
      <!-- Covers the list rather than replacing it, so the diff views stay mounted. -->
      {#if loading && files.length}
        <div class="changes-splash absolute inset-0 z-20 flex bg-background">
          <ChangesSkeleton
            label={pending?.retarget
              ? pending.target
                ? `Loading ${diffTargetLabel(pending.target)}…`
                : 'Loading uncommitted changes…'
              : 'Loading changes…'}
          />
        </div>
      {/if}
    </div>
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
