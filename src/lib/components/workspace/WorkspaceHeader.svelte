<script lang="ts">
  import { windowDrag } from '$lib/primitives/window-drag'
  import { Button } from '$lib/components/ui/button'
  import { repos, shellDialogs, threads } from '$lib/state'
  import { appControls, controlApp } from '$lib/run-app/actions'
  import { isWorkspaceProvisioning } from '$lib/state/activity'
  import { isHeaderShortcut } from '$lib/keyboard/header-shortcuts'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import GitConflictBanner from './GitConflictBanner.svelte'
  import WorkspaceBranchTag from './WorkspaceBranchTag.svelte'
  import WorkspaceMoreMenu from './WorkspaceMoreMenu.svelte'
  import WorkspacePrimaryAction from './WorkspacePrimaryAction.svelte'
  import EarlierThreadsPicker from './EarlierThreadsPicker.svelte'
  import WorkspaceTrayButtons from './WorkspaceTrayButtons.svelte'
  import WorkspaceRunControls from './WorkspaceRunControls.svelte'
  import { formatCost, totalCost } from '$lib/usage/usage'
  import {
    openWorkspaceTerminal,
    startBranchReview,
  } from '$lib/workspace/worktree-actions'
  import Pencil from '@lucide/svelte/icons/pencil'

  let {
    workspace,
    titleRef = $bindable(),
    onRename,
  }: {
    workspace: Workspace
    titleRef?: HTMLHeadingElement | undefined
    onRename: () => void
  } = $props()

  const repo = $derived(repos.getById(workspace.repoId))
  const runtime = $derived(workspace.runtime)
  const wsThreads = $derived(threads.forWorkspace(workspace.id))
  // What this work has cost, the threads it started included.
  const cost = $derived(
    totalCost(
      [
        ...wsThreads,
        ...wsThreads.flatMap((thread) => threads.childrenOf(thread.id)),
      ].map((thread) => thread.usage),
    ),
  )
  const provisioning = $derived(isWorkspaceProvisioning(workspace.lifecycle))
  const setupUnfinished = $derived(
    provisioning || workspace.lifecycle === 'provisioningFailed',
  )
  // A review runs in the workspace's one thread, so it waits for that thread to finish.
  const reviewing = $derived(
    wsThreads.some((thread) => thread.status === 'running'),
  )

  // Handled here rather than on the buttons, since most actions live in ⋯.
  function onKeydown(event: KeyboardEvent) {
    if (isHeaderShortcut(event, 'runApp')) {
      event.preventDefault()
      const { start } = appControls(workspace.id)
      if (start) void controlApp(workspace.id, start.action)
    } else if (isHeaderShortcut(event, 'reviewBranch')) {
      event.preventDefault()
      if (workspace.kind !== 'review' && !setupUnfinished && !reviewing) {
        void startBranchReview(workspace.id)
      }
    } else if (isHeaderShortcut(event, 'stopApp')) {
      event.preventDefault()
      void controlApp(workspace.id, 'stop')
    } else if (isHeaderShortcut(event, 'openTerminal')) {
      event.preventDefault()
      void openWorkspaceTerminal(workspace.id)
    } else if (isHeaderShortcut(event, 'deleteWorkspace')) {
      event.preventDefault()
      shellDialogs.open('teardown', { workspaceId: workspace.id })
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<header
  class="flex flex-col"
  data-tauri-drag-region
  data-od-id="ws-header"
  use:windowDrag
>
  <div class="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
    <div class="flex min-w-0 flex-1 items-center gap-2">
      <h1
        bind:this={titleRef}
        tabindex="-1"
        class="min-w-24 truncate text-[15px] font-medium tracking-tight outline-none"
      >
        {workspace.name}
      </h1>
      <Button
        variant="ghost"
        size="icon-sm"
        class="size-6 shrink-0 text-muted-foreground"
        aria-label="Rename workspace"
        onclick={onRename}
      >
        <Pencil class="size-3.5" aria-hidden="true" />
      </Button>
      <WorkspaceBranchTag
        workspaceId={workspace.id}
        repoId={workspace.repoId}
        branch={workspace.branch}
        {provisioning}
      />
      {#if repo?.name}
        <p class="truncate text-xs text-muted-foreground">{repo.name}</p>
      {/if}
      <EarlierThreadsPicker workspaceId={workspace.id} />
      {#if cost != null}
        <p
          class="shrink-0 font-mono text-xs text-muted-foreground tabular-nums"
          data-od-id="ws-cost"
          title="What this workspace's agents have cost so far"
        >
          {formatCost(cost)}
        </p>
      {/if}
    </div>

    <div
      class="flex flex-wrap items-center justify-end gap-2 max-md:w-full"
      data-od-id="ws-actions"
    >
      <WorkspaceTrayButtons {workspace} />
      <WorkspaceRunControls workspaceId={workspace.id} />
      <WorkspaceMoreMenu
        workspaceId={workspace.id}
        canReview={workspace.kind !== 'review'}
        reviewDisabled={setupUnfinished || reviewing}
      />
      <WorkspacePrimaryAction {workspace} />
    </div>
  </div>
  {#if runtime.conflict}
    <GitConflictBanner
      workspaceId={workspace.id}
      branch={workspace.branch}
      base={workspace.base}
      conflict={runtime.conflict}
    />
  {/if}
</header>
