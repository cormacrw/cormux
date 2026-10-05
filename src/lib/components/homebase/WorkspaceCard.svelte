<script lang="ts">
  import { Badge } from '$lib/components/ui/badge'
  import { clayColor } from '$lib/clay/identity'
  import Buddy from '$lib/components/clay/Buddy.svelte'
  import {
    buildCardDetailParts,
    formatCreatedAge,
    formatModifiedFiles,
  } from '$lib/homebase/card-details'
  import {
    workspaceCardBadgeKind,
    workspaceCardMetaText,
  } from '$lib/homebase/card-status'
  import { app, homebaseUi, settings, threads, workspaces } from '$lib/state'
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

  const isReview = $derived(workspace.kind === 'review')
  const face = $derived(
    badgeKind === 'needsAttention'
      ? 'gasp'
      : badgeKind === 'working'
        ? 'happy'
        : isReview
          ? 'happy'
          : 'sleepy',
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
  <div
    class="flex items-center gap-4 py-4 pr-4 pl-5 felt transition-[box-shadow,transform,opacity] duration-200 ease-[var(--squish)] {exiting
      ? 'pointer-events-none opacity-0'
      : deleting
        ? 'opacity-60'
        : 'group-hover/ws:-translate-y-0.5 group-hover/ws:shadow-lift-3'}"
  >
    <Buddy
      color={isReview ? 'var(--clay-plum)' : clayColor(workspace.id)}
      face={face as 'gasp' | 'happy' | 'sleepy'}
      size={45}
      breathe={badgeKind === 'working' && !settings.reduceMotion}
    />
    <div class="grid min-w-0 flex-1 gap-1.5">
      <div class="flex items-start justify-between gap-3">
        <h3
          id="ws-title-{workspace.id}"
          class="min-w-0 truncate font-display text-[16px] leading-[1.15] font-extrabold"
        >
          {workspace.name}
        </h3>
        {#if deleting}
          <Badge variant="destructive" class="shrink-0">
            <LoaderCircle
              class="size-3 {settings.reduceMotion ? '' : 'animate-spin'}"
              aria-hidden="true"
            />
            Deleting…
          </Badge>
        {:else}
          <Badge
            variant="secondary"
            class="shrink-0 {badgeKind === 'needsAttention'
              ? 'bg-marigold'
              : badgeKind === 'working'
                ? 'bg-leaf'
                : 'bg-secondary text-secondary-foreground'}"
          >
            {badgeText}
          </Badge>
        {/if}
      </div>
      <p
        class="flex flex-wrap gap-x-3 gap-y-0.5 text-[13px] font-semibold text-foreground/85"
      >
        {#each detailParts as part, index (part.kind + index)}
          {#if part.kind === 'review'}
            <span>Reviewing <span class="font-bold">#{part.number}</span></span>
          {:else if part.kind === 'branch'}
            <span
              >Branch: <code class="font-mono text-[14px] font-semibold"
                >{part.name}</code
              ></span
            >
          {:else if part.kind === 'agents'}
            <span>{part.count} {part.count === 1 ? 'agent' : 'agents'}</span>
          {:else if part.kind === 'files'}
            <span>{formatModifiedFiles(part.count)}</span>
          {:else if part.kind === 'created'}
            <span class="basis-full">{formatCreatedAge(part.fromMs, nowMs)}</span>
          {/if}
        {/each}
      </p>
    </div>
  </div>
</button>
