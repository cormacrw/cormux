<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { runWorkspaceApp } from '$lib/command-palette/actions'
  import {
    claimWorkspacePopover,
    releaseWorkspacePopover,
  } from '$lib/keyboard/popover-registry'
  import { registerPopoverCloser } from '$lib/keyboard/global-shortcuts'
  import { isHeaderShortcut } from '$lib/keyboard/header-shortcuts'
  import { shellDialogs, workspaceRecords, workspaceUi } from '$lib/state'
  import {
    openWorkspaceTerminal,
    startBranchReview,
  } from '$lib/workspace/worktree-actions'
  import Ellipsis from '@lucide/svelte/icons/ellipsis'
  import Play from '@lucide/svelte/icons/play'
  import ScanSearch from '@lucide/svelte/icons/scan-search'
  import Square from '@lucide/svelte/icons/square'
  import SquareTerminal from '@lucide/svelte/icons/square-terminal'
  import Terminal from '@lucide/svelte/icons/terminal'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import { onMount } from 'svelte'

  // Narrow windows only: the header's secondary buttons fold in here.
  let {
    workspaceId,
    runDisabled = false,
    canReview = false,
    reviewDisabled = false,
  }: {
    workspaceId: string
    runDisabled?: boolean
    canReview?: boolean
    reviewDisabled?: boolean
  } = $props()

  let open = $state(false)
  let triggerEl = $state<HTMLButtonElement | null>(null)

  const appStatus = $derived(workspaceRecords.runtime(workspaceId).appStatus)
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

  function onKeydown(event: KeyboardEvent) {
    if (!isHeaderShortcut(event, 'j')) return
    event.preventDefault()
    open = !open
  }
</script>

<svelte:window onkeydown={onKeydown} />

<DropdownMenu.Root bind:open>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <Button
        {...props}
        bind:ref={triggerEl}
        variant="secondary"
        size="xl"
        class="w-12 px-0"
        aria-label="More workspace actions"
        aria-keyshortcuts="Meta+J"
        title="More actions (⌘J)"
        aria-haspopup="menu"
        aria-controls="ws-more"
        data-ws-focus="more"
        data-od-id="ws-more"
      >
        <Ellipsis class="size-4" aria-hidden="true" />
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content align="end" id="ws-more" class="w-60">
    <DropdownMenu.Group>
      <DropdownMenu.Label class="text-xs text-muted-foreground">
        App
      </DropdownMenu.Label>
      <DropdownMenu.Item
        onclick={() => closeAndRun(() => workspaceUi.toggleOutputTab())}
      >
        <Terminal class="size-4" aria-hidden="true" />
        Output
      </DropdownMenu.Item>
      {#if appStatus === 'stopped' || appStatus === 'crashed'}
        <DropdownMenu.Item
          disabled={appStatus === 'stopped' && runDisabled}
          onclick={() =>
            runWorkspaceApp(
              workspaceId,
              appStatus === 'crashed' ? 'restart' : 'run',
            )}
        >
          <Play class="size-4" aria-hidden="true" />
          Run
          <DropdownMenu.Shortcut>⌘R</DropdownMenu.Shortcut>
        </DropdownMenu.Item>
      {:else}
        <DropdownMenu.Item onclick={() => runWorkspaceApp(workspaceId, 'stop')}>
          <Square class="size-4" aria-hidden="true" />
          Stop app
          <DropdownMenu.Shortcut>⌘.</DropdownMenu.Shortcut>
        </DropdownMenu.Item>
      {/if}
    </DropdownMenu.Group>
    <DropdownMenu.Separator />
    <DropdownMenu.Item
      onclick={() => closeAndRun(() => void openWorkspaceTerminal(workspaceId))}
    >
      <SquareTerminal class="size-4" aria-hidden="true" />
      Open in Terminal
      <DropdownMenu.Shortcut>⌘T</DropdownMenu.Shortcut>
    </DropdownMenu.Item>
    {#if canReview}
      <DropdownMenu.Item
        disabled={reviewDisabled}
        onclick={() => closeAndRun(() => void startBranchReview(workspaceId))}
      >
        <ScanSearch class="size-4" aria-hidden="true" />
        Review changes
        <DropdownMenu.Shortcut>⌘I</DropdownMenu.Shortcut>
      </DropdownMenu.Item>
    {/if}
    <DropdownMenu.Item
      variant="destructive"
      onclick={() => closeAndRun(() => shellDialogs.openTeardown(workspaceId))}
    >
      <Trash2 class="size-4" aria-hidden="true" />
      Delete workspace…
      <DropdownMenu.Shortcut>⌘D</DropdownMenu.Shortcut>
    </DropdownMenu.Item>
  </DropdownMenu.Content>
</DropdownMenu.Root>
