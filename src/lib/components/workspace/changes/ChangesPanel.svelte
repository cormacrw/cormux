<script lang="ts">
  import { commands } from '$lib/ipc'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import {
    changesReview,
    threadTimeline,
    workspaceDiff,
    workspaceUi,
  } from '$lib/state'
  import { formatChangeCounts } from '$lib/workspace/diff-totals'
  import { inferFileStatus, statusLabel } from '$lib/changes/file-status'
  import { applyFileReview } from '$lib/changes/review-actions'
  import { selectedFile } from '$lib/changes/select-file'
  import { Button } from '$lib/components/ui/button'
  import { Badge } from '$lib/components/ui/badge'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import ChangesDiffBody from './ChangesDiffBody.svelte'
  import ChangesFileList from './ChangesFileList.svelte'
  import { cn } from '$lib/utils'
  import Check from '@lucide/svelte/icons/check'
  import Clock from '@lucide/svelte/icons/clock'
  import File from '@lucide/svelte/icons/file'
  import List from '@lucide/svelte/icons/list'
  import X from '@lucide/svelte/icons/x'

  let {
    workspace,
    thread,
    overlay = false,
  }: {
    workspace: Workspace
    thread: Thread
    overlay?: boolean
  } = $props()

  const files = $derived(workspaceDiff.filesByWorkspace[workspace.id] ?? [])
  const totals = $derived(workspaceDiff.totals(workspace.id))
  const countLabel = $derived(formatChangeCounts(totals))

  const activeFile = $derived(selectedFile(files, workspaceUi.selectedDiffPath))
  const selectedPath = $derived(activeFile?.path ?? null)

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

  let reviewBusy = $state(false)
  let diffScrollTop = $state(0)
  let diffBodyEl: HTMLDivElement | undefined = $state()

  $effect(() => {
    const paths = new Set(files.map((file) => file.path))
    changesReview.pruneMissing(workspace.id, paths)
  })

  $effect(() => {
    void workspace.branch
    void commands.refreshWorkspaceDiff(workspace.id).catch(() => {})
  })

  $effect(() => {
    void workspace.id
    workspaceUi.selectedDiffPath = null
  })

  $effect(() => {
    void selectedPath
    queueMicrotask(() => {
      if (diffBodyEl) diffBodyEl.scrollTop = 0
    })
  })

  function closePanel() {
    workspaceUi.changesOpen = false
  }

  function selectPath(path: string) {
    workspaceUi.selectedDiffPath = path
    workspaceUi.changesOpen = true
  }

  function setDiffMode(mode: 'unified' | 'split') {
    if (diffBodyEl) diffScrollTop = diffBodyEl.scrollTop
    workspaceUi.diffMode = mode
    queueMicrotask(() => {
      if (diffBodyEl) diffBodyEl.scrollTop = diffScrollTop
    })
  }

  async function review(decision: 'approve' | 'reject' | 'undo') {
    if (!selectedPath || reviewBusy) return
    reviewBusy = true
    try {
      await applyFileReview(workspace.id, selectedPath, decision)
    } catch (error) {
      console.warn('review failed', error)
    } finally {
      reviewBusy = false
    }
  }

  const fileReview = $derived(
    selectedPath ? changesReview.review(workspace.id, selectedPath) : 'pending',
  )
</script>

<aside
  id="changes"
  aria-label="Changes"
  data-od-id="changes-panel"
  class={cn(
    'flex min-h-0 flex-col border-border bg-background',
    overlay
      ? 'absolute top-0 right-0 bottom-0 z-20 w-full max-w-[520px] border-l shadow-lg'
      : 'min-w-[380px] border-l',
  )}
>
  <div class="flex h-11 shrink-0 items-center gap-2 border-b border-border/60 px-4">
    <span class="text-sm font-semibold">Changes</span>
    {#if countLabel}
      <span class="font-mono text-xs text-muted-foreground">{countLabel}</span>
    {/if}
    <span class="flex-1"></span>
    <Button variant="ghost" size="icon-sm" aria-label="Close changes" onclick={closePanel}>
      <X class="size-4" aria-hidden="true" />
    </Button>
  </div>

  <ChangesFileList
    workspaceId={workspace.id}
    {files}
    {selectedPath}
    onSelect={selectPath}
  />

  <div class="flex min-h-0 flex-1 flex-col border-t border-border/40">
    <div class="flex shrink-0 flex-col gap-2 border-b border-border/40 px-3 py-2">
      {#if activeFile}
        {@const status = inferFileStatus(activeFile)}
        <div class="flex min-w-0 flex-wrap items-center gap-2 text-sm">
          <File class="size-4 shrink-0 text-muted-foreground" aria-hidden="true" />
          <span class="min-w-0 truncate font-mono text-xs" title={activeFile.path}>
            {activeFile.path}
          </span>
          <Badge
            variant="outline"
            class={cn(
              status === 'M' && 'border-[var(--warning)]/40 text-[var(--warning)]',
              status === 'A' && 'border-[var(--success)]/40 text-[var(--success)]',
              status === 'D' && 'border-destructive/40 text-destructive',
            )}
          >
            {statusLabel[status]}
          </Badge>
          <span class="font-mono text-xs text-muted-foreground">
            {#if activeFile.added > 0}+{activeFile.added}{/if}
            {#if activeFile.deleted > 0} −{activeFile.deleted}{/if}
          </span>
        </div>
        <div class="flex flex-wrap items-center justify-between gap-2">
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
          <div class="flex flex-wrap items-center gap-2">
            {#if fileReview === 'approved'}
              <Badge variant="outline" class="border-[var(--success)]/40 text-[var(--success)]">
                <Check class="size-3" aria-hidden="true" /> Approved
              </Badge>
              <Button variant="secondary" size="sm" disabled={reviewBusy} onclick={() => review('undo')}>
                Undo
              </Button>
            {:else if fileReview === 'rejected'}
              <Badge variant="outline" class="border-destructive/40 text-destructive">
                <X class="size-3" aria-hidden="true" /> Rejected
              </Badge>
              <Button variant="secondary" size="sm" disabled={reviewBusy} onclick={() => review('undo')}>
                Undo
              </Button>
            {:else}
              <Button
                variant="destructive"
                size="sm"
                disabled={reviewBusy}
                onclick={() => review('reject')}
              >
                Reject
              </Button>
              <Button
                variant="default"
                size="sm"
                class="bg-[var(--success)] text-background hover:bg-[var(--success)]/90"
                disabled={reviewBusy}
                onclick={() => review('approve')}
              >
                <Check class="size-4" aria-hidden="true" /> Approve file
              </Button>
            {/if}
          </div>
        </div>
      {:else}
        <div class="flex items-center gap-2 text-sm text-muted-foreground">
          <File class="size-4" aria-hidden="true" />
          <span>No changes</span>
        </div>
      {/if}
    </div>

    {#if activeFile}
      <div class="min-h-0 flex flex-1 flex-col">
        {#key selectedPath}
          <ChangesDiffBody file={activeFile} bind:scrollEl={diffBodyEl} />
        {/key}
      </div>
    {:else}
      <div class="flex flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
        {#if planIdle}
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
</aside>
