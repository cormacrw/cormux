<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { Button } from '$lib/components/ui/button'
  import {
    repos,
    threads,
    workspaceDiff,
    workspaceRecords,
    workspaceUi,
  } from '$lib/state'
  import { formatChangeCounts } from '$lib/workspace/diff-totals'
  import {
    rememberHeaderFocusKey,
    restoreHeaderFocus,
  } from '$lib/workspace/header-focus'
  import { isWorkspaceProvisioning } from '$lib/workspace/provisioning'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import GitConflictBanner from './GitConflictBanner.svelte'
  import WorkspaceBranchTag from './WorkspaceBranchTag.svelte'
  import WorkspaceMoreMenu from './WorkspaceMoreMenu.svelte'
  import WorkspacePrimaryAction from './WorkspacePrimaryAction.svelte'
  import WorkspaceRunControls from './WorkspaceRunControls.svelte'
  import File from '@lucide/svelte/icons/file'
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

  let narrow = $state(false)

  const record = $derived(workspaceRecords.getRecord(workspace.id))
  const repo = $derived(record ? repos.getById(record.repoId) : undefined)
  const runtime = $derived(workspaceRecords.runtime(workspace.id))
  const wsThreads = $derived(threads.forWorkspace(workspace.id))
  const provisioning = $derived(isWorkspaceProvisioning(workspace.lifecycle))
  const runDisabled = $derived(
    provisioning || workspace.lifecycle === 'provisioningFailed',
  )

  const diffTotals = $derived(workspaceDiff.totals(workspace.id))
  const changeCountLabel = $derived(formatChangeCounts(diffTotals))

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
      workspaceUi.changesOpen,
      workspaceUi.activeTab,
      changeCountLabel,
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

  function onTitleFocus() {
    rememberHeaderFocusKey('title')
  }

  function toggleChanges() {
    workspaceUi.changesOpen = !workspaceUi.changesOpen
  }

  function startHeaderDrag(event: MouseEvent) {
    if (event.button !== 0) return
    const target = event.target
    if (!(target instanceof Element)) return
    if (target.closest('button, a, input, textarea, [role="tab"]')) return
    void getCurrentWindow().startDragging()
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<header
  class="flex flex-col"
  data-tauri-drag-region
  data-od-id="ws-header"
  onmousedown={startHeaderDrag}
>
  <div class="flex flex-wrap items-center justify-between gap-3 px-4 py-2">
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
    {#if !narrow}
      <WorkspaceRunControls
        workspaceId={workspace.id}
        repoId={record?.repoId ?? ''}
        {runDisabled}
      />

      <Button
        variant="secondary"
        size="sm"
        class="gap-2"
        aria-pressed={workspaceUi.changesOpen}
        aria-controls="changes"
        data-ws-focus="changes"
        data-od-id="ws-changes-toggle"
        onclick={toggleChanges}
      >
        <File class="size-4" aria-hidden="true" />
        Changes
        {#if changeCountLabel}
          <span class="font-mono text-xs text-muted-foreground">
            {changeCountLabel}
          </span>
        {/if}
      </Button>
    {/if}

    <WorkspaceMoreMenu
      workspaceId={workspace.id}
      base={record?.base ?? 'main'}
      behind={runtime.behind}
      ahead={runtime.ahead}
      worktreePath={record?.worktreePath ?? ''}
      {narrow}
      {runDisabled}
      {changeCountLabel}
      {onRename}
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
