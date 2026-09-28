<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import {
    pullWorkspace,
    pushWorkspace,
    rebaseWorkspace,
    requestNewThread,
    runWorkspaceApp,
  } from '$lib/command-palette/actions'
  import {
    claimWorkspacePopover,
    releaseWorkspacePopover,
  } from '$lib/keyboard/popover-registry'
  import { registerPopoverCloser } from '$lib/keyboard/global-shortcuts'
  import { shellDialogs, workspaceRecords, workspaceUi } from '$lib/state'
  import { plural } from '$lib/workspace/plural'
  import {
    openWorktreeInEditor,
    revealWorktreeInFinder,
  } from '$lib/workspace/worktree-actions'
  import Download from '@lucide/svelte/icons/download'
  import Upload from '@lucide/svelte/icons/upload'
  import Ellipsis from '@lucide/svelte/icons/ellipsis'
  import ExternalLink from '@lucide/svelte/icons/external-link'
  import File from '@lucide/svelte/icons/file'
  import FolderOpen from '@lucide/svelte/icons/folder-open'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import Pencil from '@lucide/svelte/icons/pencil'
  import Play from '@lucide/svelte/icons/play'
  import Plus from '@lucide/svelte/icons/plus'
  import RotateCw from '@lucide/svelte/icons/rotate-cw'
  import Square from '@lucide/svelte/icons/square'
  import Terminal from '@lucide/svelte/icons/terminal'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import { onMount } from 'svelte'

  let {
    workspaceId,
    base,
    behind,
    ahead,
    worktreePath,
    narrow = false,
    runDisabled = false,
    changeCountLabel = null as string | null,
    onRename,
  }: {
    workspaceId: string
    base: string
    behind: number
    ahead: number
    worktreePath: string
    narrow?: boolean
    runDisabled?: boolean
    changeCountLabel?: string | null
    onRename: () => void
  } = $props()

  let open = $state(false)
  let triggerEl = $state<HTMLButtonElement | null>(null)

  const runtime = $derived(workspaceRecords.runtime(workspaceId))
  const appStatus = $derived(runtime.appStatus)
  const pullLabel = $derived(
    behind > 0
      ? `Pull ${plural(behind, 'commit')} from ${base}`
      : `Up to date with ${base}`,
  )
  const popoverHandle = {
    close: () => {
      open = false
    },
    restoreFocus: () => {
      triggerEl?.focus()
    },
  }

  onMount(() => {
    registerPopoverCloser(() => {
      if (!open) return false
      open = false
      triggerEl?.focus()
      return true
    })
    return () => registerPopoverCloser(null)
  })

  $effect(() => {
    if (open) {
      claimWorkspacePopover(popoverHandle)
    } else {
      releaseWorkspacePopover(popoverHandle)
    }
  })

  function closeAndRun(action: () => void) {
    open = false
    action()
  }

  function requestTeardown() {
    closeAndRun(() => shellDialogs.openTeardown(workspaceId))
  }

  function requestRebase() {
    closeAndRun(() => rebaseWorkspace(workspaceId))
  }

  function toggleChanges() {
    closeAndRun(() => {
      workspaceUi.changesOpen = !workspaceUi.changesOpen
    })
  }

  function toggleOutput() {
    closeAndRun(() => workspaceUi.toggleOutputTab())
  }
</script>

<DropdownMenu.Root bind:open>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <Button
        {...props}
        bind:ref={triggerEl}
        variant="outline"
        size="icon-sm"
        aria-label="More workspace actions"
        aria-haspopup="menu"
        aria-controls="ws-more"
        data-ws-focus="more"
        data-od-id="ws-more"
      >
        <Ellipsis class="size-4" aria-hidden="true" />
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content align="end" id="ws-more" class="w-64">
    {#if narrow}
      <DropdownMenu.Group>
        <DropdownMenu.Label class="text-xs text-muted-foreground">
          App
        </DropdownMenu.Label>
        <DropdownMenu.Item onclick={toggleOutput}>
          <Terminal class="size-4" aria-hidden="true" />
          Output
        </DropdownMenu.Item>
        {#if appStatus === 'stopped'}
          <DropdownMenu.Item
            disabled={runDisabled}
            onclick={() => runWorkspaceApp(workspaceId, 'run')}
          >
            <Play class="size-4" aria-hidden="true" />
            Run
          </DropdownMenu.Item>
        {:else}
          <DropdownMenu.Item
            disabled={appStatus === 'starting'}
            onclick={() => runWorkspaceApp(workspaceId, 'restart')}
          >
            <RotateCw class="size-4" aria-hidden="true" />
            Restart app
          </DropdownMenu.Item>
          <DropdownMenu.Item
            onclick={() => runWorkspaceApp(workspaceId, 'stop')}
          >
            <Square class="size-4" aria-hidden="true" />
            Stop app
          </DropdownMenu.Item>
        {/if}
        <DropdownMenu.Item
          aria-pressed={workspaceUi.changesOpen}
          onclick={toggleChanges}
        >
          <File class="size-4" aria-hidden="true" />
          Changes
          {#if changeCountLabel}
            <span class="ml-auto font-mono text-xs text-muted-foreground">
              {changeCountLabel}
            </span>
          {/if}
        </DropdownMenu.Item>
      </DropdownMenu.Group>
      <DropdownMenu.Separator />
    {/if}

    <DropdownMenu.Item
      disabled={behind <= 0}
      onclick={() => closeAndRun(() => pullWorkspace(workspaceId))}
    >
      <Download class="size-4" aria-hidden="true" />
      {pullLabel}
    </DropdownMenu.Item>

    <DropdownMenu.Item
      disabled={behind <= 0}
      onclick={requestRebase}
      data-od-id="ws-rebase"
    >
      <GitBranch class="size-4" aria-hidden="true" />
      Rebase branch
    </DropdownMenu.Item>

    <DropdownMenu.Item
      disabled={ahead <= 0}
      onclick={() => closeAndRun(() => pushWorkspace(workspaceId))}
      data-od-id="ws-push"
    >
      <Upload class="size-4" aria-hidden="true" />
      {ahead > 0 ? `Push ${plural(ahead, 'commit')} to origin` : 'Nothing to push'}
    </DropdownMenu.Item>

    <DropdownMenu.Item
      onclick={() => closeAndRun(() => requestNewThread(workspaceId))}
    >
      <Plus class="size-4" aria-hidden="true" />
      New thread
    </DropdownMenu.Item>

    <DropdownMenu.Separator />

    <DropdownMenu.Item
      disabled={!worktreePath}
      onclick={() => closeAndRun(() => void openWorktreeInEditor(worktreePath))}
    >
      <ExternalLink class="size-4" aria-hidden="true" />
      Open in editor
    </DropdownMenu.Item>
    <DropdownMenu.Item
      disabled={!worktreePath}
      onclick={() =>
        closeAndRun(() => void revealWorktreeInFinder(worktreePath))}
    >
      <FolderOpen class="size-4" aria-hidden="true" />
      Reveal in Finder
    </DropdownMenu.Item>

    <DropdownMenu.Item onclick={() => closeAndRun(onRename)}>
      <Pencil class="size-4" aria-hidden="true" />
      Rename workspace…
    </DropdownMenu.Item>

    <DropdownMenu.Separator />

    <DropdownMenu.Item variant="destructive" onclick={requestTeardown}>
      <Trash2 class="size-4" aria-hidden="true" />
      Teardown worktree…
    </DropdownMenu.Item>
  </DropdownMenu.Content>
</DropdownMenu.Root>
