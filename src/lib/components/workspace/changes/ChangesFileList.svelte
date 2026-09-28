<script lang="ts">
  import type { DiffFile } from '$lib/ipc/bindings'
  import { changesReview } from '$lib/state'
  import { inferFileStatus, splitPath, statusLabel } from '$lib/changes/file-status'
  import { cn } from '$lib/utils'
  import Check from '@lucide/svelte/icons/check'
  import X from '@lucide/svelte/icons/x'

  let {
    workspaceId,
    files,
    selectedPath,
    onSelect,
  }: {
    workspaceId: string
    files: DiffFile[]
    selectedPath: string | null
    onSelect: (path: string) => void
  } = $props()
</script>

{#if files.length}
  <ul class="flex flex-col" aria-label="Changed files">
    {#each files as file (file.path)}
      {@const status = inferFileStatus(file)}
      {@const parts = splitPath(file.path)}
      {@const review = changesReview.review(workspaceId, file.path)}
      <li>
        <button
          type="button"
          class={cn(
            'flex w-full items-center gap-2 border-b border-border/40 px-3 py-2 text-left text-sm hover:bg-muted/40',
            selectedPath === file.path && 'bg-muted/60',
          )}
          aria-current={selectedPath === file.path ? 'true' : undefined}
          title={file.path}
          onclick={() => onSelect(file.path)}
        >
          <span class="min-w-0 flex-1 truncate">
            {#if parts.dir}
              <span class="text-muted-foreground">{parts.dir}</span>
            {/if}
            <span class="font-medium">{parts.name}</span>
          </span>
          {#if review === 'approved'}
            <Check class="size-3.5 shrink-0 text-[var(--success)]" aria-hidden="true" />
            <span class="sr-only">approved</span>
          {:else if review === 'rejected'}
            <X class="size-3.5 shrink-0 text-destructive" aria-hidden="true" />
            <span class="sr-only">rejected</span>
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
      </li>
    {/each}
  </ul>
{/if}
