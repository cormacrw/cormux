<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import { startReviewWorkspace } from '$lib/review/start-review'
  import { app, workspaceRecords, workspaces } from '$lib/state'
  import type { PullRequest } from '$lib/state/prs.svelte'
  import Check from '@lucide/svelte/icons/check'
  import Clock from '@lucide/svelte/icons/clock'
  import Glasses from '@lucide/svelte/icons/glasses'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import X from '@lucide/svelte/icons/x'
  import { openUrl } from '@tauri-apps/plugin-opener'

  let {
    pr,
    nowMs,
  }: {
    pr: PullRequest
    nowMs: number
  } = $props()

  const reviewWorkspace = $derived(
    workspaces.items.find((row) => {
      if (row.kind !== 'review' || row.prNumber !== pr.num) return false
      if (!pr.repoId) return true
      return workspaceRecords.getRecord(row.id)?.repoId === pr.repoId
    }),
  )

  const relationshipLabel = $derived.by(() => {
    switch (pr.rel) {
      case 'review':
        return 'Review requested'
      case 'author':
        return 'Opened by you'
      case 'assigned':
        return 'Assigned to you'
      default:
        return 'Mentioned'
    }
  })

  // The bead says how the checks are doing; drafts stay pebble.
  const beadColor = $derived.by(() => {
    if (pr.isDraft) return 'var(--clay-pebble)'
    if (pr.checks === 'running') return 'var(--clay-marigold)'
    if (pr.checks === 'fail') return 'var(--clay-brick)'
    return 'var(--clay-leaf)'
  })

  const checksChip = $derived(
    pr.checks === 'pass'
      ? 'bg-mint'
      : pr.checks === 'fail'
        ? 'bg-felt-brick text-brick-ink'
        : 'bg-butter',
  )

  const checksLabel = $derived.by(() => {
    if (pr.checks === 'pass') return 'Checks passing'
    if (pr.checks === 'fail') {
      const count = pr.failing ?? 1
      return count === 1 ? '1 check failing' : `${count} checks failing`
    }
    return 'Checks running'
  })

  const reviewLabel = $derived(
    reviewWorkspace
      ? `Open the review workspace for #${pr.num}`
      : `Review #${pr.num} in a new workspace`,
  )

  let starting = $state(false)

  async function onReview() {
    // One review workspace per PR: reopen it rather than starting a duplicate.
    if (reviewWorkspace) {
      app.openWorkspace(reviewWorkspace.id)
      return
    }
    starting = true
    try {
      await startReviewWorkspace(pr)
    } finally {
      starting = false
    }
  }

  function onTitleClick(event: MouseEvent) {
    event.preventDefault()
    void openUrl(pr.htmlUrl)
  }
</script>

<li
  class="grid gap-x-3 gap-y-1.5 rounded-[16px_12px_14px_11px] px-3 py-2 transition-colors hover:bg-foreground/[0.04] min-[1181px]:grid-cols-[auto_1fr_auto_auto] min-[1181px]:items-center min-[721px]:max-[1180px]:grid-cols-[auto_1fr_auto]"
  data-od-id="pr-row-{pr.num}"
>
  <div class="flex items-start min-[1181px]:self-center">
    <span
      class="bead mt-0.5 size-6 min-[1181px]:mt-0"
      style="background: {beadColor}"
      aria-hidden="true"
    ></span>
    <span class="sr-only"
      >{pr.isDraft ? 'Draft pull request' : 'Open pull request'}</span
    >
  </div>

  <div class="min-w-0 space-y-0.5">
    <h3 class="font-sans text-[15px] leading-snug font-bold">
      <a
        href={pr.htmlUrl}
        class="rounded-sm hover:underline focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
        onclick={onTitleClick}
      >
        {pr.title}
        <span class="text-muted-foreground"> #{pr.num}</span>
      </a>
    </h3>
    <div
      class="flex flex-wrap items-center gap-x-1.5 gap-y-0.5 text-[13px] font-semibold text-muted-foreground [&>span+span]:before:mr-1.5 [&>span+span]:before:content-['·']"
    >
      <span class={pr.rel === 'review' ? 'font-medium text-foreground' : ''}>
        {relationshipLabel}
      </span>
      {#if pr.author !== 'you'}
        <span>@{pr.author}</span>
      {/if}
      <span>
        <code class="font-mono text-[12.5px]">{pr.head}</code>
        →
        <code class="font-mono text-[12.5px]">{pr.base}</code>
      </span>
      <span>
        {pr.files} files
        <span class="text-success">+{pr.additions}</span>
        <span class="text-destructive"> −{pr.deletions}</span>
      </span>
      <span>Updated {formatRelativeAge(pr.updatedAtMs, nowMs)}</span>
      {#if pr.repoFullName}
        <span class="text-muted-foreground/80">{pr.repoFullName}</span>
      {/if}
    </div>

    {#if pr.checks !== 'none'}
      <div
        class="flex flex-wrap items-center gap-2 min-[1181px]:hidden min-[721px]:max-[1180px]:flex"
      >
        <span
          class="inline-flex h-6 items-center gap-1 rounded-full px-2.5 text-[12px] font-bold text-cocoa shadow-[inset_0_-2px_0_rgb(0_0_0/0.08)] {checksChip}"
        >
          {#if pr.checks === 'pass'}
            <Check class="size-3.5" strokeWidth={3} aria-hidden="true" />
          {:else if pr.checks === 'fail'}
            <X class="size-3.5" strokeWidth={3} aria-hidden="true" />
          {:else}
            <Clock class="size-3.5" aria-hidden="true" />
          {/if}
          {checksLabel}
        </span>
      </div>
    {/if}
  </div>

  <div
    class="hidden min-[1181px]:flex flex-col items-end gap-2 min-[721px]:max-[1180px]:hidden"
  >
    {#if pr.checks !== 'none'}
      <span
        class="inline-flex h-6 items-center gap-1 rounded-full px-2.5 text-[12px] font-bold text-cocoa shadow-[inset_0_-2px_0_rgb(0_0_0/0.08)] {checksChip}"
      >
        {#if pr.checks === 'pass'}
          <Check class="size-3.5" strokeWidth={3} aria-hidden="true" />
        {:else if pr.checks === 'fail'}
          <X class="size-3.5" strokeWidth={3} aria-hidden="true" />
        {:else}
          <Clock class="size-3.5" aria-hidden="true" />
        {/if}
        {checksLabel}
      </span>
    {/if}
  </div>

  <div
    class="min-[1181px]:self-center min-[721px]:max-[1180px]:row-span-2 min-[721px]:max-[1180px]:self-center"
  >
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <Button
            {...props}
            size="sm"
            class="bg-plum text-cocoa"
            onclick={() => void onReview()}
            disabled={starting}
            aria-busy={starting}
            aria-label={reviewLabel}
          >
            {#if starting}
              <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
            {:else}
              <Glasses class="size-4" strokeWidth={2.5} aria-hidden="true" />
            {/if}
            {reviewWorkspace ? 'Open' : 'Review'}
          </Button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content side="left">{reviewLabel}</Tooltip.Content>
    </Tooltip.Root>
  </div>
</li>
