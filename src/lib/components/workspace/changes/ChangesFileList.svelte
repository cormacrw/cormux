<script lang="ts">
  import type { DiffFile } from '$lib/ipc/bindings'
  import { diffComments } from '$lib/changes/diff-comments.svelte'
  import {
    inferFileStatus,
    isDiffCollapsed,
    splitPath,
    statusLabel,
  } from '$lib/changes/file-status'
  import { workspaceUi } from '$lib/state'
  import { cn } from '$lib/utils'
  import ChangesDiffView from './ChangesDiffView.svelte'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import MessageSquare from '@lucide/svelte/icons/message-square'

  let {
    workspaceId,
    branch,
    files,
  }: {
    workspaceId: string
    branch: string | null
    files: DiffFile[]
  } = $props()

  function toggle(path: string) {
    workspaceUi.collapsedDiffPaths = {
      ...workspaceUi.collapsedDiffPaths,
      [path]: !isDiffCollapsed(workspaceUi.collapsedDiffPaths, path),
    }
  }
</script>

<ul class="flex flex-col" aria-label="Changed files">
  {#each files as file (file.path)}
    {@const status = inferFileStatus(file)}
    {@const parts = splitPath(file.path)}
    {@const open = !isDiffCollapsed(workspaceUi.collapsedDiffPaths, file.path)}
    {@const commentCount = diffComments.forFile(
      workspaceId,
      branch,
      file.path,
    ).length}
    {@const bodyId = `changes-file-${file.path.replace(/[^\w-]/g, '_')}`}
    <li class="border-b border-border/60" data-diff-path={file.path}>
      <button
        type="button"
        class="sticky top-0 z-10 flex w-full items-center gap-2 border-b border-border/40 bg-background px-3 py-2 text-left text-sm hover:bg-[color-mix(in_oklch,var(--muted)_40%,var(--background))]"
        aria-expanded={open}
        aria-controls={bodyId}
        title={file.path}
        onclick={() => toggle(file.path)}
      >
        <ChevronRight
          class={cn(
            'size-4 shrink-0 text-muted-foreground transition-transform',
            open && 'rotate-90',
          )}
          aria-hidden="true"
        />
        <span class="min-w-0 flex-1 truncate">
          {#if parts.dir}
            <span class="text-muted-foreground">{parts.dir}</span>
          {/if}
          <span class="font-medium">{parts.name}</span>
        </span>
        {#if commentCount}
          <span class="flex shrink-0 items-center gap-0.5 text-xs text-info">
            <MessageSquare class="size-3.5" aria-hidden="true" />
            {commentCount}
            <span class="sr-only"
              >{commentCount === 1 ? 'comment' : 'comments'}</span
            >
          </span>
        {/if}
        <span class="shrink-0 font-mono text-xs">
          {#if file.added > 0}
            <span class="text-[var(--success)]">+{file.added}</span>
          {/if}
          {#if file.deleted > 0}
            <span class="text-destructive">−{file.deleted}</span>
          {/if}
        </span>
        <span
          class={cn(
            'shrink-0 font-mono text-xs font-semibold',
            status === 'A' && 'text-[var(--success)]',
            status === 'D' && 'text-destructive',
            status === 'M' && 'text-[var(--warning)]',
          )}
        >
          <span aria-hidden="true">{status}</span>
          <span class="sr-only">{statusLabel[status]}</span>
        </span>
      </button>
      {#if open}
        <div id={bodyId}>
          {#if file.hunks.length}
            <ChangesDiffView {workspaceId} {branch} {file} />
          {:else}
            <p class="px-3 py-2 text-xs text-muted-foreground">
              No text changes to show.
            </p>
          {/if}
        </div>
      {/if}
    </li>
  {/each}
</ul>
