<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { appControls, controlApp } from '$lib/run-app/actions'
  import {
    claimWorkspacePopover,
    registerPopoverCloser,
    releaseWorkspacePopover,
  } from '$lib/keyboard/popover-registry'
  import { shortcutAria, shortcutLabel } from '$lib/keyboard/shortcuts'
  import { shellDialogs, workspaceUi } from '$lib/state'
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

  let {
    workspaceId,
    canReview = false,
    reviewDisabled = false,
  }: {
    workspaceId: string
    canReview?: boolean
    reviewDisabled?: boolean
  } = $props()

  let open = $state(false)
  let triggerEl = $state<HTMLButtonElement | null>(null)

  const controls = $derived(appControls(workspaceId))
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
    if (!shellDialogs.allows(event, 'moreActions')) return
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
        size="default"
        class="w-8 px-0"
        aria-label="More workspace actions"
        aria-keyshortcuts={shortcutAria('moreActions')}
        title="More actions ({shortcutLabel('moreActions')})"
        aria-haspopup="menu"
        aria-controls="ws-more"
        data-od-id="ws-more"
      >
        <Ellipsis class="size-4" aria-hidden="true" />
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content align="end" id="ws-more" class="w-60">
    <DropdownMenu.Group>
      <DropdownMenu.Item
        onclick={() => closeAndRun(() => workspaceUi.toggleTray('output'))}
      >
        <Terminal class="size-4" aria-hidden="true" />
        Output
        <DropdownMenu.Shortcut
          >{shortcutLabel('toggleOutput')}</DropdownMenu.Shortcut
        >
      </DropdownMenu.Item>
      {#if controls.start}
        {@const { action, enabled } = controls.start}
        <DropdownMenu.Item
          disabled={!enabled}
          onclick={() => void controlApp(workspaceId, action)}
        >
          <Play class="size-4" aria-hidden="true" />
          Run
          <DropdownMenu.Shortcut
            >{shortcutLabel('runApp')}</DropdownMenu.Shortcut
          >
        </DropdownMenu.Item>
      {:else}
        <DropdownMenu.Item onclick={() => void controlApp(workspaceId, 'stop')}>
          <Square class="size-4" aria-hidden="true" />
          Stop app
          <DropdownMenu.Shortcut
            >{shortcutLabel('stopApp')}</DropdownMenu.Shortcut
          >
        </DropdownMenu.Item>
      {/if}
    </DropdownMenu.Group>
    <DropdownMenu.Separator />
    <DropdownMenu.Item
      onclick={() => closeAndRun(() => void openWorkspaceTerminal(workspaceId))}
    >
      <SquareTerminal class="size-4" aria-hidden="true" />
      Open in Terminal
      <DropdownMenu.Shortcut
        >{shortcutLabel('openTerminal')}</DropdownMenu.Shortcut
      >
    </DropdownMenu.Item>
    {#if canReview}
      <DropdownMenu.Item
        disabled={reviewDisabled}
        onclick={() => closeAndRun(() => void startBranchReview(workspaceId))}
      >
        <ScanSearch class="size-4" aria-hidden="true" />
        Review changes
        <DropdownMenu.Shortcut
          >{shortcutLabel('reviewBranch')}</DropdownMenu.Shortcut
        >
      </DropdownMenu.Item>
    {/if}
    <DropdownMenu.Item
      variant="destructive"
      onclick={() =>
        closeAndRun(() => shellDialogs.open('teardown', { workspaceId }))}
    >
      <Trash2 class="size-4" aria-hidden="true" />
      Delete workspace…
      <DropdownMenu.Shortcut
        >{shortcutLabel('deleteWorkspace')}</DropdownMenu.Shortcut
      >
    </DropdownMenu.Item>
  </DropdownMenu.Content>
</DropdownMenu.Root>
