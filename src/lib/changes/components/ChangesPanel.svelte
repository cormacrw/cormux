<script lang="ts">
  import { sendToThread } from '$lib/thread/actions'
  import { pluralWord } from '$lib/format/plural'
  import TrayControls from '$lib/components/tray/TrayControls.svelte'
  import { onMount, tick, untrack } from 'svelte'
  import type { Action } from 'svelte/action'
  import { afterPaint } from '$lib/changes/mount-queue'
  import { commands } from '$lib/ipc'
  import { coreErrorText } from '$lib/core/core-error'
  import { startsOver } from '$lib/core/owner-scoped'
  import {
    commentBranch,
    diffComments,
  } from '$lib/changes/diff-comments.svelte'
  import { formatCommentsForAgent } from '$lib/changes/diff-comment-format'

  import { threads } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { nav } from '$lib/core/nav.svelte'
  import { changesUi } from '$lib/changes/changes-ui.svelte'
  import { openWorkspaceThread } from '$lib/nav/open'
  import { workspaceDiff } from '$lib/changes/diff-store.svelte'
  import { formatChangeCounts } from '$lib/changes/diff-totals'
  import { revealDiffLine, scrollWithin } from '$lib/changes/reveal-line'
  import { Button } from '$lib/components/ui/button'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { diffTargetLabel } from '$lib/changes/stack'
  import ChangesFileList from './ChangesFileList.svelte'
  import ChangesSkeleton from './ChangesSkeleton.svelte'
  import StackRail from './StackRail.svelte'
  import DiscardChangesDialog, {
    type DiscardRequest,
  } from './DiscardChangesDialog.svelte'
  import ChevronsDownUp from '@lucide/svelte/icons/chevrons-down-up'
  import ChevronsUpDown from '@lucide/svelte/icons/chevrons-up-down'
  import GitCompare from '@lucide/svelte/icons/git-compare'
  import MopSparkles from '@lucide/svelte/icons/mop-sparkles'
  import Send from '@lucide/svelte/icons/send'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert'
  import Undo2 from '@lucide/svelte/icons/undo-2'

  let { workspace }: { workspace: Workspace } = $props()

  const files = $derived(workspaceDiff.filesByWorkspace[workspace.id] ?? [])
  const totals = $derived(workspaceDiff.totals(workspace.id))
  const countLabel = $derived(formatChangeCounts(totals))
  // Stale files give way to the splash on a retarget; a refresh keeps what is on screen.
  const pending = $derived(workspaceDiff.pending(workspace.id))
  const loading = $derived(
    !!pending && (pending.retarget || files.length === 0),
  )
  // While a retarget loads, the label shows where it is headed.
  const shownTarget = $derived(workspaceDiff.shownTarget(workspace.id))

  const allCollapsed = $derived(
    files.length > 0 && files.every((file) => changesUi.isCollapsed(file)),
  )

  let scrollEl: HTMLDivElement | undefined = $state()

  // Only uncommitted work can be thrown away; a branch's commits stay put.
  const canDiscard = $derived(
    !pending?.retarget &&
      workspaceDiff.target(workspace.id) === null &&
      files.length > 0,
  )
  let discardRequest = $state<DiscardRequest | null>(null)

  // The tab switches the moment it's clicked: the skeleton paints first and the file
  // list, which can be long, mounts in the frame after.
  let listReady = $state(false)
  onMount(() => afterPaint(() => (listReady = true)))

  // Snapshots rebuild the workspace object, so key the refresh on its id and branch alone.
  const workspaceId = $derived(workspace.id)
  const branch = $derived(workspace.branch)

  // What the core said when a refresh last failed, for the note shown instead of a clean diff.
  let refreshError = $state<string | null>(null)
  const failed = $derived(workspaceDiff.failed(workspace.id))

  function refresh(id: string) {
    refreshError = null
    void workspaceDiff.fetch(id, async () => {
      const result = await commands.refreshWorkspaceDiff(id)
      if (result.status === 'ok') return true
      if (id === workspaceId) refreshError = coreErrorText(result.error)
      return false
    })
  }

  $effect(() => {
    void branch
    const id = workspaceId
    untrack(() => refresh(id))
  })

  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  const fileRows = new Map<string, HTMLElement>()
  const trackFile: Action<HTMLElement, string> = (node, path) => {
    fileRows.set(path, node)
    return {
      destroy: () => {
        if (fileRows.get(path) === node) fileRows.delete(path)
      },
    }
  }

  // A file link (conversation, findings) expands that file and scrolls to it, or to its line.
  $effect(() => {
    const reveal = nav.route.view === 'workspace' ? nav.route.reveal : null
    const target = reveal && files.find((file) => file.path === reveal.path)
    // Waits for the list to mount, which a first open of the panel holds back a frame.
    const scroller = scrollEl
    if (!reveal || !target || !scroller) return
    const { path, line } = reveal
    untrack(() => {
      nav.take('workspace', 'reveal')
      if (changesUi.isCollapsed(target)) changesUi.setCollapsed([target], false)
    })
    void tick().then(() => {
      const fileEl = fileRows.get(path)
      if (!fileEl) return
      scrollWithin(scroller, fileEl, 'start')
      if (line != null) revealDiffLine(scroller, fileEl, line)
    })
  })

  function setAllCollapsed(collapsed: boolean) {
    changesUi.setCollapsed(files, collapsed)
  }

  function setDiffMode(mode: 'unified' | 'split') {
    const top = scrollEl?.scrollTop ?? 0
    changesUi.diffMode = mode
    void tick().then(() => {
      if (scrollEl) scrollEl.scrollTop = top
    })
  }

  // Comments belong to the level on screen, not to whatever is mid-load.
  const commentScope = $derived(
    commentBranch(workspaceDiff.target(workspace.id), workspace.branch),
  )
  const comments = $derived(diffComments.forBranch(workspace.id, commentScope))
  let sending = $derived(startsOver(workspaceId, false))
  // Whichever thread is on screen, only the workspace's live one can take the comments.
  const liveThread = $derived(threads.liveFor(workspaceId))
  // The agent works on the checked-out branch, so comments on another level wait until it's checked out.
  const offBranch = $derived(
    commentScope !== null && commentScope !== workspace.branch,
  )

  async function sendComments() {
    if (!comments.length || sending || offBranch || !liveThread) return
    const id = workspaceId
    const threadId = liveThread.id
    sending = true
    const sent = comments
    diffComments.clear(id, commentScope)
    if (await sendToThread(threadId, formatCommentsForAgent(sent))) {
      // Show the conversation so the agent's reply is in view, unless you've moved on.
      if (id === workspaceId) openWorkspaceThread(id, threadId)
    } else {
      for (const comment of sent) diffComments.add(id, comment)
    }
    if (id === workspaceId) sending = false
  }
</script>

<div
  id="changes-panel"
  role="region"
  aria-label="Git"
  data-od-id="changes-panel"
  class="flex min-h-0 flex-1"
>
  <StackRail {workspace} />
  <div class="flex min-h-0 min-w-0 flex-1 flex-col">
    <div
      class="flex shrink-0 flex-wrap items-center gap-2 border-b border-border/60 px-3 py-2"
    >
      <span
        class="flex h-6 min-w-0 items-center gap-1.5 font-mono text-xs"
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
          value={changesUi.diffMode}
          onValueChange={(value) => {
            if (value === 'unified' || value === 'split') setDiffMode(value)
          }}
          variant="outline"
          size="sm"
          aria-label="Diff layout"
        >
          <ToggleGroup.Item
            value="unified"
            aria-pressed={changesUi.diffMode === 'unified'}
          >
            Unified
          </ToggleGroup.Item>
          <ToggleGroup.Item
            value="split"
            aria-pressed={changesUi.diffMode === 'split'}
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
      {#if canDiscard}
        <Button
          variant="ghost"
          size="sm"
          class="gap-1.5 text-muted-foreground hover:text-destructive"
          data-od-id="changes-discard-all"
          onclick={() => (discardRequest = { file: null, count: files.length })}
        >
          <Undo2 class="size-3.5" aria-hidden="true" /> Discard all
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
            ? `These comments are on ${commentScope}. Check it out to send them to ${liveThread?.role ?? 'the agent'}.`
            : undefined}
        >
          <Button
            size="sm"
            class="gap-1.5"
            disabled={sending || offBranch || !liveThread}
            onclick={() => void sendComments()}
          >
            <Send class="size-3.5" aria-hidden="true" />
            Send {comments.length}
            {pluralWord(comments.length, 'comment')} to {liveThread?.role ??
              'the agent'}
          </Button>
        </span>
      {/if}
      <TrayControls tray="changes" name="Git" shortcut="toggleGit" />
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
            scroller={scrollEl}
            {trackFile}
            onDiscard={canDiscard
              ? (file) => (discardRequest = { file, count: 1 })
              : undefined}
          />
        </div>
      {:else if !loading && failed}
        <div
          class="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center"
          data-od-id="changes-refresh-failed"
        >
          <TriangleAlert
            class="size-8 text-muted-foreground"
            aria-hidden="true"
          />
          <div>
            <h2 class="text-sm font-semibold">Couldn’t load the diff</h2>
            <p class="text-sm text-muted-foreground">
              {refreshError ?? 'Git didn’t answer.'}
            </p>
          </div>
          <Button
            variant="secondary"
            size="sm"
            onclick={() => refresh(workspaceId)}
          >
            Try again
          </Button>
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
        <div class="changes-splash absolute inset-0 z-20 flex bg-popover">
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

<DiscardChangesDialog
  workspaceId={workspace.id}
  request={discardRequest}
  onClose={() => (discardRequest = null)}
/>

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
