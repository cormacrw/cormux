<script lang="ts">
  import { Badge } from '$lib/components/ui/badge'
  import * as Card from '$lib/components/ui/card'
  import {
    buildCardDetailParts,
    formatCreatedAge,
    formatModifiedFiles,
  } from '$lib/homebase/card-details'
  import {
    workspaceCardBadgeKind,
    workspaceCardMetaText,
  } from '$lib/homebase/card-status'
  import {
    app,
    homebaseUi,
    repos,
    settings,
    threads,
    workspaceRecords,
    workspaces,
  } from '$lib/state'
  import Folder from '@lucide/svelte/icons/folder'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { onMount } from 'svelte'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'

  let {
    workspace,
    nowMs,
    entering,
    exiting,
    onExitComplete,
  }: {
    workspace: Workspace
    nowMs: number
    entering: boolean
    exiting: boolean
    onExitComplete: () => void
  } = $props()

  const workspaceThreads = $derived(threads.forWorkspace(workspace.id))
  const repoName = $derived(
    repos.getById(workspaceRecords.getRecord(workspace.id)?.repoId ?? '')?.name,
  )
  const deleting = $derived(workspaces.isDeleting(workspace))
  const badgeKind = $derived(
    workspaceCardBadgeKind(workspace, workspaceThreads),
  )
  const badgeText = $derived(workspaceCardMetaText(workspace, workspaceThreads))
  const detailParts = $derived(
    buildCardDetailParts({
      branch: workspace.branch,
      agentCount: workspaceThreads.length,
      modifiedFiles: workspace.modifiedFiles,
      createdAtMs: workspace.createdAtMs,
      reviewPr: workspace.prNumber,
      nowMs,
    }),
  )

  onMount(() => {
    if (exiting) {
      const ms = settings.reduceMotion ? 0 : 150
      const timer = setTimeout(onExitComplete, ms)
      return () => clearTimeout(timer)
    }
    if (settings.reduceMotion) {
      homebaseUi.markCardSeen(workspace.id)
    }
    return undefined
  })

  function openWorkspace() {
    if (deleting) return
    app.openWorkspace(workspace.id)
  }
</script>

<button
  type="button"
  aria-labelledby="ws-title-{workspace.id}"
  aria-disabled={deleting}
  class="group/ws block min-w-0 w-full {deleting
    ? 'cursor-default'
    : 'cursor-pointer'} border-0 bg-transparent p-0 text-left font-[inherit] text-inherit {entering &&
  !settings.reduceMotion
    ? 'animate-in fade-in duration-300'
    : ''} {exiting && !settings.reduceMotion
    ? 'animate-out fade-out zoom-out-98 duration-150'
    : ''}"
  onclick={openWorkspace}
  onanimationend={(event) => {
    if (exiting && event.animationName.includes('fade-out')) {
      onExitComplete()
    }
    if (entering) homebaseUi.markCardSeen(workspace.id)
  }}
>
  <Card.Root
    class="transition-[box-shadow,ring-color,opacity] {exiting
      ? 'pointer-events-none opacity-0'
      : deleting
        ? 'opacity-60'
        : 'hover:ring-foreground/20'}"
  >
    <Card.Header class="gap-3">
      <div class="flex items-start justify-between gap-3">
        <Card.Title
          id="ws-title-{workspace.id}"
          class="min-w-0 truncate text-base font-medium leading-snug"
        >
          {workspace.name}
        </Card.Title>
        {#if deleting}
          <Badge
            variant="outline"
            class="shrink-0 border-destructive/30 bg-destructive/5 text-destructive"
          >
            <LoaderCircle
              class="size-3 {settings.reduceMotion ? '' : 'animate-spin'}"
              aria-hidden="true"
            />
            Deleting…
          </Badge>
        {:else}
          <Badge
            variant="outline"
            class={badgeKind === 'needsAttention'
              ? 'shrink-0 border-amber-500/40 bg-amber-500/10 text-amber-800 dark:text-amber-200'
              : badgeKind === 'working'
                ? 'shrink-0 border-emerald-500/40 bg-emerald-500/10 text-emerald-800 dark:text-emerald-200'
                : 'shrink-0 text-muted-foreground'}
          >
            {#if badgeKind === 'working'}
              <span
                class="size-1.5 rounded-full bg-emerald-500 {settings.reduceMotion
                  ? ''
                  : 'animate-pulse'}"
                aria-hidden="true"
              ></span>
            {/if}
            {badgeText}
          </Badge>
        {/if}
      </div>
      <p
        class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground"
      >
        {#if repoName}
          <span class="flex items-center gap-1" title="Repository">
            <Folder class="size-3.5 shrink-0" aria-hidden="true" />
            <span class="sr-only">Repository</span>{repoName}
          </span>
        {/if}
        {#each detailParts.filter((part) => part.kind !== 'agents') as part, index (part.kind + index)}
          {#if part.kind === 'review'}
            <span
              >Reviewing <code class="font-mono text-[11px]"
                >#{part.number}</code
              ></span
            >
          {:else if part.kind === 'branch'}
            <span class="flex min-w-0 items-center gap-1" title="Branch">
              <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
              <span class="sr-only">Branch</span><code
                class="truncate font-mono text-[11px]">{part.name}</code
              >
            </span>
          {:else if part.kind === 'files'}
            <span>{formatModifiedFiles(part.count)}</span>
          {:else if part.kind === 'created'}
            <span>{formatCreatedAge(part.fromMs, nowMs)}</span>
          {/if}
        {/each}
      </p>
    </Card.Header>
  </Card.Root>
</button>
