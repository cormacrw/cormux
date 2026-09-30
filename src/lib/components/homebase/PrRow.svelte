<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import { startReviewWorkspace } from '$lib/review/start-review'
  import { app, workspaceRecords, workspaces } from '$lib/state'
  import type { PullRequest } from '$lib/state/prs.svelte'
  import Check from '@lucide/svelte/icons/check'
  import Clock from '@lucide/svelte/icons/clock'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'
  import Glasses from '@lucide/svelte/icons/glasses'
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

  const iconTone = $derived.by(() => {
    if (pr.isDraft) return 'text-muted-foreground'
    if (pr.checks === 'running') return 'text-amber-500 dark:text-amber-400'
    return 'text-emerald-600 dark:text-emerald-400'
  })

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

  function onReview() {
    // One review workspace per PR: reopen it rather than starting a duplicate.
    if (reviewWorkspace) {
      app.openWorkspace(reviewWorkspace.id)
      return
    }
    void startReviewWorkspace(pr)
  }

  function onTitleClick(event: MouseEvent) {
    event.preventDefault()
    void openUrl(pr.htmlUrl)
  }
</script>

<li
  class="grid gap-3 px-4 py-3 min-[1181px]:grid-cols-[auto_1fr_auto_auto] min-[1181px]:items-center min-[721px]:max-[1180px]:grid-cols-[auto_1fr_auto]"
  data-od-id="pr-row-{pr.num}"
>
  <div class="flex items-start min-[1181px]:self-center">
    <GitPullRequest class="size-4 shrink-0 {iconTone}" aria-hidden="true" />
    <span class="sr-only"
      >{pr.isDraft ? 'Draft pull request' : 'Open pull request'}</span
    >
  </div>

  <div class="min-w-0 space-y-1.5">
    <h3 class="text-sm font-medium leading-snug">
      <a
        href={pr.htmlUrl}
        class="rounded-sm hover:underline focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
        onclick={onTitleClick}
      >
        {pr.title}
        <span class="font-normal text-muted-foreground"> #{pr.num}</span>
      </a>
    </h3>
    <div
      class="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground"
    >
      <span class={pr.rel === 'review' ? 'font-medium text-foreground' : ''}>
        {relationshipLabel}
      </span>
      {#if pr.author !== 'you'}
        <span>@{pr.author}</span>
      {/if}
      <span>
        <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]"
          >{pr.head}</code
        >
        →
        <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]"
          >{pr.base}</code
        >
      </span>
      <span>
        {pr.files} files
        <span class="text-emerald-600 dark:text-emerald-400"
          >+{pr.additions}</span
        >
        <span class="text-red-600 dark:text-red-400"> −{pr.deletions}</span>
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
          class="inline-flex items-center gap-1 text-xs text-muted-foreground"
        >
          {#if pr.checks === 'pass'}
            <Check class="size-3.5 text-emerald-600" aria-hidden="true" />
          {:else if pr.checks === 'fail'}
            <X class="size-3.5 text-red-600" aria-hidden="true" />
          {:else}
            <Clock class="size-3.5" aria-hidden="true" />
          {/if}
          {checksLabel}
        </span>
      </div>
    {/if}
  </div>

  <div
    class="hidden min-[1181px]:flex flex-col items-end gap-2 text-xs text-muted-foreground min-[721px]:max-[1180px]:hidden"
  >
    {#if pr.checks !== 'none'}
      <span class="inline-flex items-center gap-1">
        {#if pr.checks === 'pass'}
          <Check class="size-3.5 text-emerald-600" aria-hidden="true" />
        {:else if pr.checks === 'fail'}
          <X class="size-3.5 text-red-600" aria-hidden="true" />
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
            variant="ghost"
            size="icon-sm"
            onclick={onReview}
            aria-label={reviewLabel}
          >
            <Glasses class="size-4" aria-hidden="true" />
          </Button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content side="left">{reviewLabel}</Tooltip.Content>
    </Tooltip.Root>
  </div>
</li>
