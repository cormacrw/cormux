<script lang="ts">
  import * as Empty from '$lib/components/ui/empty'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import type { PrFilter } from '$lib/homebase/pr-filter'
  import { homebaseUi, prs } from '$lib/state'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'
  import PrRow from './PrRow.svelte'

  const nowMs = $derived(Date.now() + homebaseUi.ageTick * 0)

  const syncLabel = $derived.by(() => {
    if (!prs.authConfigured) return null
    if (prs.syncedAtMs == null) return 'Syncing from GitHub…'
    return `Synced from GitHub ${formatRelativeAge(prs.syncedAtMs, nowMs)}`
  })
</script>

<section
  class="grid gap-4"
  aria-labelledby="open-prs-title"
  data-od-id="open-prs"
>
  <div class="flex flex-wrap items-end justify-between gap-3">
    <h2 id="open-prs-title" class="text-sm font-medium tracking-tight">
      Open pull requests
      <span class="font-mono text-muted-foreground">{prs.count}</span>
    </h2>
    <div class="flex flex-wrap items-center gap-3">
      {#if syncLabel}
        <p
          class="inline-flex items-center gap-1.5 text-xs text-muted-foreground"
          data-od-id="pr-sync"
        >
          <GitPullRequest class="size-3.5" aria-hidden="true" />
          {syncLabel}
        </p>
      {/if}
      <ToggleGroup.Root
        type="single"
        value={prs.filter}
        onValueChange={(value) => {
          if (value) prs.setFilter(value as PrFilter)
        }}
        variant="outline"
        size="sm"
        role="group"
        aria-label="Filter pull requests"
        data-od-id="pr-filter"
      >
        <ToggleGroup.Item value="all" aria-pressed={prs.filter === 'all'}>
          All
          <span class="ml-1 font-mono text-muted-foreground"
            >{prs.filterCounts.all}</span
          >
        </ToggleGroup.Item>
        <ToggleGroup.Item value="review" aria-pressed={prs.filter === 'review'}>
          Review requested
          <span class="ml-1 font-mono text-muted-foreground"
            >{prs.filterCounts.review}</span
          >
        </ToggleGroup.Item>
        <ToggleGroup.Item value="author" aria-pressed={prs.filter === 'author'}>
          Yours
          <span class="ml-1 font-mono text-muted-foreground"
            >{prs.filterCounts.author}</span
          >
        </ToggleGroup.Item>
      </ToggleGroup.Root>
    </div>
  </div>

  {#if prs.isEmpty}
    <Empty.Root
      class="border border-dashed border-border/80"
      data-od-id="pr-list"
    >
      <Empty.Header>
        <Empty.Title>No open pull requests</Empty.Title>
        <Empty.Description>
          Pull requests you open, or that ask for your review, show up here.
        </Empty.Description>
      </Empty.Header>
    </Empty.Root>
  {:else if prs.filterEmpty}
    <Empty.Root
      class="border border-dashed border-border/80"
      data-od-id="pr-list"
    >
      <Empty.Header>
        <Empty.Title>Nothing in this filter</Empty.Title>
        <Empty.Description>Switch the filter to see the rest.</Empty.Description
        >
      </Empty.Header>
    </Empty.Root>
  {:else}
    <ul
      class="divide-y rounded-xl border border-border text-sm"
      data-od-id="pr-list"
    >
      {#each prs.filtered as pr (pr.id)}
        <PrRow {pr} {nowMs} />
      {/each}
    </ul>
  {/if}
</section>
