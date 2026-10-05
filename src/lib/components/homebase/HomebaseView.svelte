<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as Empty from '$lib/components/ui/empty'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import {
    homebaseFilterCounts,
    matchesHomebaseFilter,
    type HomebaseFilter,
  } from '$lib/homebase/filter'
  import { app, homebaseUi, workspaces } from '$lib/state'
  import { Kbd } from '$lib/components/ui/kbd'
  import Plus from '@lucide/svelte/icons/plus'
  import OpenPrsSection from './OpenPrsSection.svelte'
  import PinnedTodosSection from './PinnedTodosSection.svelte'
  import ScratchesSection from './ScratchesSection.svelte'
  import WorkspaceCard from './WorkspaceCard.svelte'

  let homeTitle: HTMLHeadingElement | undefined = $state()
  let nowMs = $state(Date.now())

  $effect(() => {
    if (app.focusTarget !== 'homebase') return
    void app.focusGeneration
    homeTitle?.focus()
  })

  $effect(() => {
    if (app.view !== 'homebase') return
    void homebaseUi.ageTick
    nowMs = Date.now()
    const id = window.setInterval(() => {
      homebaseUi.bumpAgeTick()
      nowMs = Date.now()
    }, 60_000)
    return () => window.clearInterval(id)
  })

  const filterCounts = $derived(
    homebaseFilterCounts(workspaces.items.map((item) => item.lifecycle)),
  )

  const visibleWorkspaces = $derived(
    workspaces.items.filter((item) =>
      matchesHomebaseFilter(item.lifecycle, homebaseUi.filter),
    ),
  )

  const exitingCards = $derived(homebaseUi.exitingCards)
</script>

<div class="view-home relative flex min-h-0 flex-1 flex-col overflow-y-auto">
  <div class="mx-auto flex w-full max-w-[1200px] flex-col gap-10 p-8">
    <header
      class="sticky top-0 z-10 -mx-8 -mt-8 -mb-4 flex flex-wrap items-center justify-between gap-4 bg-background px-8 pt-8 pb-4"
      aria-labelledby="home-title"
    >
      <h1
        bind:this={homeTitle}
        id="home-title"
        tabindex="-1"
        data-od-id="home-title"
        class="clay-title outline-none"
      >
        Homebase
      </h1>
      <div class="flex items-center gap-4">
        <Button
          variant="secondary"
          size="default"
          class="gap-2"
          data-od-id="new-session"
          onclick={() => app.requestNewScratch()}
        >
          <Plus class="size-4" strokeWidth={3.5} aria-hidden="true" />
          New scratch
          <Kbd aria-hidden="true">⌘S</Kbd>
        </Button>
        <Button
          variant="default"
          size="default"
          class="gap-2"
          onclick={() => app.requestNewWorkspace()}
        >
          <Plus class="size-4" strokeWidth={3.5} aria-hidden="true" />
          New Workspace
          <Kbd aria-hidden="true">⌘N</Kbd>
        </Button>
      </div>
    </header>

    <div
      class="grid items-start gap-10 [grid-template-columns:repeat(auto-fit,minmax(min(100%,420px),1fr))]"
    >
      <PinnedTodosSection />

      <ScratchesSection />
    </div>

    <section class="grid gap-4" aria-labelledby="ws-section-title">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h2 id="ws-section-title" class="clay-section">Workspaces</h2>
        <ToggleGroup.Root
          type="single"
          bind:value={
            () => homebaseUi.filter,
            (next) => {
              // Clicking the pressed item deselects it; keep a filter selected.
              if (next) homebaseUi.filter = next as HomebaseFilter
            }
          }
          role="group"
          aria-label="Filter workspaces"
        >
          <ToggleGroup.Item
            value="all"
            aria-pressed={homebaseUi.filter === 'all'}
          >
            All {filterCounts.all}
          </ToggleGroup.Item>
          <ToggleGroup.Item
            value="running"
            aria-pressed={homebaseUi.filter === 'running'}
          >
            Running {filterCounts.running}
          </ToggleGroup.Item>
          <ToggleGroup.Item
            value="idle"
            aria-pressed={homebaseUi.filter === 'idle'}
          >
            Idle {filterCounts.idle}
          </ToggleGroup.Item>
        </ToggleGroup.Root>
      </div>

      {#if workspaces.items.length === 0}
        <Empty.Root class="felt">
          <Empty.Header>
            <Empty.Title>No workspaces running</Empty.Title>
            <Empty.Description>
              Create one to hand a task to an agent in its own worktree.
              Homebase stays untouched until you merge.
            </Empty.Description>
          </Empty.Header>
          <Empty.Content>
            <Button
              variant="secondary"
              onclick={() => app.requestNewWorkspace()}
            >
              Create a workspace
            </Button>
          </Empty.Content>
        </Empty.Root>
      {:else if visibleWorkspaces.length === 0 && exitingCards.length === 0}
        <Empty.Root class="felt">
          <Empty.Header>
            <Empty.Title>Nothing in this filter</Empty.Title>
            <Empty.Description>
              Switch the filter to see your other workspaces.
            </Empty.Description>
          </Empty.Header>
          <Empty.Content>
            <Button
              variant="secondary"
              onclick={() => homebaseUi.resetFilter()}
            >
              Show all
            </Button>
          </Empty.Content>
        </Empty.Root>
      {:else}
        <div
          class="grid gap-4 [grid-template-columns:repeat(auto-fill,minmax(min(100%,315px),1fr))]"
        >
          {#each visibleWorkspaces as workspace (workspace.id)}
            <WorkspaceCard
              {workspace}
              {nowMs}
              entering={!homebaseUi.seenCardIds.includes(workspace.id)}
              exiting={false}
              onExitComplete={() => {}}
            />
          {/each}
          {#each exitingCards as workspace (workspace.id)}
            <WorkspaceCard
              {workspace}
              {nowMs}
              entering={false}
              exiting={true}
              onExitComplete={() => homebaseUi.finishCardExit(workspace.id)}
            />
          {/each}
        </div>
      {/if}
    </section>

    <OpenPrsSection />
  </div>
</div>

<style>
  :global(.zoom-out-98) {
    transform: scale(0.98);
  }
</style>
