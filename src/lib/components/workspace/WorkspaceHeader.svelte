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
  import { openWorkspaceTerminal } from '$lib/workspace/worktree-actions'
  import CommandIcon from '@lucide/svelte/icons/command'
  import Pencil from '@lucide/svelte/icons/pencil'
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

  const record = $derived(workspaceRecords.getRecord(workspace.id))
  const repo = $derived(record ? repos.getById(record.repoId) : undefined)
  const runtime = $derived(workspaceRecords.runtime(workspace.id))
  const wsThreads = $derived(threads.forWorkspace(workspace.id))
  const provisioning = $derived(isWorkspaceProvisioning(workspace.lifecycle))
  const runDisabled = $derived(
    provisioning || workspace.lifecycle === 'provisioningFailed',
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

  onMount(() => {
    const media = window.matchMedia('(max-width: 768px)')
    const sync = () => {
      narrow = media.matches
    }
    sync()
    media.addEventListener('change', sync)
    return () => media.removeEventListener('change', sync)
  })

  // Handled here rather than on the buttons, which narrow windows fold into ⋯.
  function onKeydown(event: KeyboardEvent) {
    const status = runtime.appStatus
    if (isHeaderShortcut(event, 'r')) {
      event.preventDefault()
      if (status === 'crashed' || status === 'running') {
        runWorkspaceApp(workspace.id, 'restart')
      } else if (
        status === 'stopped' &&
        !runDisabled &&
        repo?.runCommand?.trim()
      ) {
        runWorkspaceApp(workspace.id, 'run')
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

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<header
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
        class="min-w-0 truncate text-[15px] font-medium tracking-tight outline-none"
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
    </div>

    <div
      class="flex flex-wrap items-center justify-end gap-2 max-md:w-full"
      data-od-id="ws-actions"
    >
      {#if narrow}
        <WorkspaceMoreMenu workspaceId={workspace.id} {runDisabled} />
      {:else}
        <WorkspaceRunControls
          workspaceId={workspace.id}
          repoId={record?.repoId ?? ''}
          {runDisabled}
        />
        <Button
          variant="secondary"
          size="xl"
          aria-keyshortcuts="Meta+T"
          data-ws-focus="terminal"
          data-od-id="ws-terminal"
          onclick={() => void openWorkspaceTerminal(workspace.id)}
        >
          <SquareTerminal class="size-4" aria-hidden="true" />
          Terminal
          <Kbd class="gap-0.5" aria-hidden="true"><CommandIcon />T</Kbd>
        </Button>
        <Button
          variant="destructive"
          size="xl"
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
