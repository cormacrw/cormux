<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { Button } from '$lib/components/ui/button'
  import {
    repos,
    settings,
    shellDialogs,
    threads,
    workspaceRecords,
    workspaceUi,
  } from '$lib/state'
  import { workspaceRunCommand } from '$lib/run-app/run-command'
  import {
    rememberHeaderFocusKey,
    restoreHeaderFocus,
  } from '$lib/workspace/header-focus'
  import { isWorkspaceProvisioning } from '$lib/workspace/provisioning'
  import { isHeaderShortcut } from '$lib/keyboard/header-shortcuts'
  import { runWorkspaceApp } from '$lib/command-palette/actions'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import GitConflictBanner from './GitConflictBanner.svelte'
  import WorkspaceBranchTag from './WorkspaceBranchTag.svelte'
  import WorkspaceMoreMenu from './WorkspaceMoreMenu.svelte'
  import WorkspacePrimaryAction from './WorkspacePrimaryAction.svelte'
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

  let headerEl: HTMLElement | undefined = $state()

  const record = $derived(workspaceRecords.getRecord(workspace.id))
  const repo = $derived(record ? repos.getById(record.repoId) : undefined)
  const runtime = $derived(workspaceRecords.runtime(workspace.id))
  const wsThreads = $derived(threads.forWorkspace(workspace.id))
  const cost = $derived(totalCost(wsThreads.map((thread) => thread.usage)))
  const provisioning = $derived(isWorkspaceProvisioning(workspace.lifecycle))
  const runDisabled = $derived(
    provisioning || workspace.lifecycle === 'provisioningFailed',
  )
  const reviewing = $derived(
    wsThreads.some(
      (thread) => thread.role === 'Reviewer' && thread.status === 'running',
    ),
  )

  const headerSignature = $derived(
    [
      workspace.name,
      workspace.branch,
      workspace.lifecycle,
      workspace.modifiedFiles,
      workspace.prNumber,
      workspace.activityText,
      runtime.appStatus,
      runtime.port,
      runtime.behind,
      runtime.ahead,
      runtime.conflict,
      workspaceUi.activeTab,
    ].join('|'),
  )

  $effect(() => {
    void headerSignature
    void restoreHeaderFocus()
  })

  // Handled here rather than on the buttons, since most actions live in ⋯.
  function onKeydown(event: KeyboardEvent) {
    const status = runtime.appStatus
    if (isHeaderShortcut(event, 'r')) {
      event.preventDefault()
      if (status === 'crashed') {
        runWorkspaceApp(workspace.id, 'restart')
      } else if (
        status === 'stopped' &&
        !runDisabled &&
        workspaceRunCommand(settings.rows, workspace.id, repo?.runCommand)
      ) {
        runWorkspaceApp(workspace.id, 'run')
      }
    } else if (isHeaderShortcut(event, 'i')) {
      event.preventDefault()
      if (workspace.kind !== 'review' && !runDisabled && !reviewing) {
        void startBranchReview(workspace.id)
      }
    } else if (isHeaderShortcut(event, '.')) {
      event.preventDefault()
      if (status !== 'stopped' && status !== 'crashed') {
        runWorkspaceApp(workspace.id, 'stop')
      }
    } else if (isHeaderShortcut(event, 't')) {
      event.preventDefault()
      void openWorkspaceTerminal(workspace.id)
    } else if (isHeaderShortcut(event, 'd')) {
      event.preventDefault()
      shellDialogs.openTeardown(workspace.id)
    }
  }

  function onTitleFocus() {
    rememberHeaderFocusKey('title')
  }

  function startHeaderDrag(event: MouseEvent) {
    if (event.button !== 0) return
    const target = event.target
    if (!(target instanceof Element)) return
    if (target.closest('button, a, input, textarea, [role="tab"]')) return
    void getCurrentWindow().startDragging()
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<header
  bind:this={headerEl}
  class="flex flex-col"
  data-tauri-drag-region
  data-od-id="ws-header"
  onmousedown={startHeaderDrag}
>
  <div class="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
    <div class="flex min-w-0 flex-1 items-center gap-2">
      <h1
        bind:this={titleRef}
        tabindex="-1"
        data-ws-focus="title"
        class="min-w-24 truncate text-[15px] font-medium tracking-tight outline-none"
        onfocus={onTitleFocus}
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
        repoId={record?.repoId ?? ''}
        branch={workspace.branch}
        threads={wsThreads}
        {provisioning}
      />
      {#if repo?.name}
        <p class="truncate text-xs text-muted-foreground">{repo.name}</p>
      {/if}
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
      <WorkspaceRunControls
        workspaceId={workspace.id}
        repoId={record?.repoId ?? ''}
        {runDisabled}
      />
      <WorkspaceMoreMenu
        workspaceId={workspace.id}
        {runDisabled}
        canReview={workspace.kind !== 'review'}
        reviewDisabled={runDisabled || reviewing}
      />
      <WorkspacePrimaryAction {workspace} />
    </div>
  </div>
  {#if runtime.conflict}
    <GitConflictBanner
      workspaceId={workspace.id}
      branch={workspace.branch}
      base={record?.base ?? 'main'}
      conflict={runtime.conflict}
    />
  {/if}
</header>
