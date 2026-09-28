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
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import { ensureWorkspaceSummary } from '$lib/homebase/request-summary'
  import { app, homebaseUi, settings, threads, workspaces } from '$lib/state'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import MessageSquare from '@lucide/svelte/icons/message-square'
  import { onMount } from 'svelte'

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

  const hasSummary = $derived(Boolean(workspace.summary?.trim()))

  onMount(() => {
    if (exiting) {
      const ms = settings.reduceMotion ? 0 : 150
      const timer = setTimeout(onExitComplete, ms)
      return () => clearTimeout(timer)
    }
    if (settings.reduceMotion) {
      homebaseUi.markCardSeen(workspace.id)
    }
    void ensureWorkspaceSummary(workspace, (patch) => {
      workspaces.patchCard(workspace.id, patch)
    })
    return undefined
  })

  function openWorkspace() {
    app.openWorkspace(workspace.id)
  }
</script>

<button
  type="button"
  aria-labelledby="ws-title-{workspace.id}"
  class="group/ws block min-w-0 w-full cursor-pointer border-0 bg-transparent p-0 text-left font-[inherit] text-inherit {entering &&
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
    class="transition-[box-shadow,ring-color] hover:ring-foreground/20 {exiting
      ? 'pointer-events-none opacity-0'
      : ''}"
  >
    <Card.Header class="gap-3 border-b border-border/60 pb-3">
      <div class="flex items-start justify-between gap-3">
        <Card.Title
          id="ws-title-{workspace.id}"
          class="min-w-0 truncate text-base font-medium leading-snug"
        >
          {workspace.name}
        </Card.Title>
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
      </div>
      <p class="flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
        {#each detailParts as part, index (part.kind + index)}
          {#if part.kind === 'review'}
            <span
              >Reviewing <code class="font-mono text-[11px]"
                >#{part.number}</code
              ></span
            >
          {:else if part.kind === 'branch'}
            <span
              >Branch: <code class="font-mono text-[11px]">{part.name}</code
              ></span
            >
          {:else if part.kind === 'agents'}
            <span>{part.count} {part.count === 1 ? 'agent' : 'agents'}</span>
          {:else if part.kind === 'files'}
            <span>{formatModifiedFiles(part.count)}</span>
          {:else if part.kind === 'created'}
            <span>{formatCreatedAge(part.fromMs, nowMs)}</span>
          {/if}
        {/each}
      </p>
    </Card.Header>
    <Card.Content class="grid gap-3 pt-3">
      <div class="grid gap-2" aria-busy={hasSummary ? undefined : true}>
        {#if hasSummary}
          <p class="line-clamp-3 text-sm leading-relaxed text-muted-foreground">
            {workspace.summary}
          </p>
          <p class="flex items-center gap-1.5 text-xs text-muted-foreground/80">
            <MessageSquare class="size-3 opacity-70" aria-hidden="true" />
            Summarized by {workspace.summarySource}{workspace.summaryAtMs
              ? `, ${formatRelativeAge(workspace.summaryAtMs, nowMs)}`
              : ''}
          </p>
        {:else}
          <div class="grid gap-2" aria-hidden="true">
            <div
              class="h-3 rounded-md bg-muted/80 {settings.reduceMotion
                ? ''
                : 'animate-pulse'}"
            ></div>
            <div
              class="h-3 w-[70%] rounded-md bg-muted/60 {settings.reduceMotion
                ? ''
                : 'animate-pulse'}"
            ></div>
          </div>
          <p class="flex items-center gap-1.5 text-xs text-muted-foreground/80">
            <MessageSquare class="size-3 opacity-70" aria-hidden="true" />
            Summarizing with Haiku 4.5…
          </p>
        {/if}
      </div>
    </Card.Content>
  </Card.Root>
</button>
