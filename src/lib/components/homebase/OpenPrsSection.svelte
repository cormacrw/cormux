<script lang="ts">
  import * as Empty from '$lib/components/ui/empty'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import type { PrFilter } from '$lib/homebase/pr-filter'
  import { groupPrsByStack, prCards } from '$lib/homebase/pr-stacks'
  import { homebaseUi, prs } from '$lib/state'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'
  import Layers from '@lucide/svelte/icons/layers'
  import PrRow from './PrRow.svelte'

  const nowMs = $derived(Date.now() + homebaseUi.ageTick * 0)

  const cards = $derived(prCards(groupPrsByStack(prs.filtered)))

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
    <h2 id="open-prs-title" class="clay-section">Open pull requests</h2>
    <div class="flex flex-wrap items-center gap-3">
      {#if syncLabel}
        <p
          class="inline-flex items-center gap-1.5 text-[13px] font-semibold text-ground-ink"
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
        role="group"
        aria-label="Filter pull requests"
        data-od-id="pr-filter"
      >
        <ToggleGroup.Item value="all" aria-pressed={prs.filter === 'all'}>
          All
          <span class="ml-0.5"
            >{prs.filterCounts.all}</span
          >
        </ToggleGroup.Item>
        <ToggleGroup.Item value="review" aria-pressed={prs.filter === 'review'}>
          Review requested
          <span class="ml-0.5"
            >{prs.filterCounts.review}</span
          >
        </ToggleGroup.Item>
        <ToggleGroup.Item value="author" aria-pressed={prs.filter === 'author'}>
          Yours
          <span class="ml-0.5"
            >{prs.filterCounts.author}</span
          >
        </ToggleGroup.Item>
      </ToggleGroup.Root>
    </div>
  </div>

  {#if prs.isEmpty}
    <Empty.Root class="felt" data-od-id="pr-list">
      <Empty.Header>
        <Empty.Title>No open pull requests</Empty.Title>
        <Empty.Description>
          Pull requests you open, or that ask for your review, show up here.
        </Empty.Description>
      </Empty.Header>
    </Empty.Root>
  {:else if prs.filterEmpty}
    <Empty.Root class="felt" data-od-id="pr-list">
      <Empty.Header>
        <Empty.Title>Nothing in this filter</Empty.Title>
        <Empty.Description>Switch the filter to see the rest.</Empty.Description
        >
      </Empty.Header>
    </Empty.Root>
  {:else}
    <div class="grid gap-3" data-od-id="pr-list">
      {#each cards as card (`${card.kind}-${card.id}`)}
        {#if card.kind === 'list'}
          <ul class="felt grid gap-0.5 px-3 py-3 text-sm">
            {#each card.prs as pr (pr.id)}
              <PrRow {pr} {nowMs} />
            {/each}
          </ul>
        {:else}
          <section
            class="felt overflow-hidden px-3 py-3 text-sm"
            aria-labelledby="pr-stack-label-{card.id}"
            data-od-id="pr-stack-{card.id}"
          >
            <header
              class="mx-1 mb-1.5 flex flex-wrap items-center gap-x-2 gap-y-1 rounded-[14px_12px_14px_11px] bg-butter px-3 py-2 text-[12px] font-semibold text-cocoa"
            >
              <Layers class="size-4" strokeWidth={2.5} aria-hidden="true" />
              <h3
                id="pr-stack-label-{card.id}"
                class="font-display text-[14px] font-extrabold"
              >
                Stack of {card.prs.length}
              </h3>
              <span>
                onto
                <code class="font-mono text-[12.5px] font-semibold"
                  >{card.trunk}</code
                >
              </span>
              {#if card.prs[0]?.repoFullName}
                <span class="ml-auto opacity-75"
                  >{card.prs[0].repoFullName}</span
                >
              {/if}
            </header>
            <ul class="grid gap-1" aria-labelledby="pr-stack-label-{card.id}">
              {#each card.prs as pr (pr.id)}
                <PrRow {pr} {nowMs} />
              {/each}
            </ul>
          </section>
        {/if}
      {/each}
    </div>
  {/if}
</section>
