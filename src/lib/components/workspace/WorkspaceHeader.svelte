<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { Button } from '$lib/components/ui/button'
  import {
    repos,
    shellDialogs,
    threads,
    workspaceRecords,
    workspaceUi,
  } from '$lib/state'
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
  import { Kbd } from '$lib/components/ui/kbd'
  import {
    openWorkspaceTerminal,
    startBranchReview,
  } from '$lib/workspace/worktree-actions'
  import CommandIcon from '@lucide/svelte/icons/command'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import Pencil from '@lucide/svelte/icons/pencil'
  import ScanSearch from '@lucide/svelte/icons/scan-search'
  import SquareTerminal from '@lucide/svelte/icons/square-terminal'
  import Trash2 from '@lucide/svelte/icons/trash-2'

  let {
    workspace,
    titleRef = $bindable(),
    onRename,
  }: {
    workspace: Workspace
    titleRef?: HTMLHeadingElement | undefined
    onRename: () => void
  } = $props()

  let narrow = $state(false)
  let headerEl: HTMLElement | undefined = $state()

  const record = $derived(workspaceRecords.getRecord(workspace.id))
  const repo = $derived(record ? repos.getById(record.repoId) : undefined)
  const runtime = $derived(workspaceRecords.runtime(workspace.id))
  const wsThreads = $derived(threads.forWorkspace(workspace.id))
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

  // The secondary buttons fold into ⋯ once the header can't fit them beside the title.
  // Measured on the header, not the window, since the sidebar takes a varying share.
  const NARROW_HEADER_PX = 1240
  onMount(() => {
    if (!headerEl) return
    const observer = new ResizeObserver(([entry]) => {
      if (entry) narrow = entry.contentRect.width < NARROW_HEADER_PX
    })
    observer.observe(headerEl)
    return () => observer.disconnect()
  })

  // Handled here rather than on the buttons, which narrow windows fold into ⋯.
  function onKeydown(event: KeyboardEvent) {
    const status = runtime.appStatus
    if (isHeaderShortcut(event, 'r')) {
      event.preventDefault()
      if (status === 'crashed') {
        runWorkspaceApp(workspace.id, 'restart')
      } else if (
        status === 'stopped' &&
        !runDisabled &&
        repo?.runCommand?.trim()
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
  class="flex flex-col gap-2 px-4 pt-3"
  data-tauri-drag-region
  data-od-id="ws-header"
  onmousedown={startHeaderDrag}
>
  <div class="felt-sm flex flex-wrap items-center justify-between gap-3 px-4 py-2.5">
    <div class="flex min-w-0 flex-1 items-center gap-2">
      <h1
        bind:this={titleRef}
        tabindex="-1"
        data-ws-focus="title"
        class="clay-title min-w-24 truncate outline-none"
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
        <p class="truncate text-sm font-bold text-muted-foreground">{repo.name}</p>
      {/if}
    </div>

    <div
      class="flex flex-wrap items-center justify-end gap-2 max-md:w-full"
      data-od-id="ws-actions"
    >
      {#if narrow}
        <WorkspaceMoreMenu
          workspaceId={workspace.id}
          {runDisabled}
          canReview={workspace.kind !== 'review'}
          reviewDisabled={runDisabled || reviewing}
        />
      {:else}
        <WorkspaceRunControls
          workspaceId={workspace.id}
          repoId={record?.repoId ?? ''}
          {runDisabled}
        />
        <Button
          variant="secondary"
          size="default"
          aria-keyshortcuts="Meta+T"
          data-ws-focus="terminal"
          data-od-id="ws-terminal"
          onclick={() => void openWorkspaceTerminal(workspace.id)}
        >
          <SquareTerminal class="size-4" aria-hidden="true" />
          Terminal
          <Kbd class="gap-0.5" aria-hidden="true"><CommandIcon />T</Kbd>
        </Button>
        {#if workspace.kind !== 'review'}
          <Button
            variant="secondary"
            size="default"
            disabled={runDisabled || reviewing}
            aria-keyshortcuts="Meta+I"
            title="Review this branch's changes, uncommitted work included"
            data-ws-focus="review"
            data-od-id="ws-review"
            onclick={() => void startBranchReview(workspace.id)}
          >
            {#if reviewing}
              <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
              Reviewing…
            {:else}
              <ScanSearch class="size-4" aria-hidden="true" />
              Review
              <Kbd class="gap-0.5" aria-hidden="true"><CommandIcon />I</Kbd>
            {/if}
          </Button>
        {/if}
        <Button
          variant="destructive"
          size="default"
          aria-label="Delete workspace"
          aria-keyshortcuts="Meta+D"
          data-ws-focus="delete"
          data-od-id="ws-delete"
          onclick={() => shellDialogs.openTeardown(workspace.id)}
        >
          <Trash2 class="size-4" aria-hidden="true" />
          Delete
          <Kbd class="gap-0.5" aria-hidden="true"><CommandIcon />D</Kbd>
        </Button>
      {/if}

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
