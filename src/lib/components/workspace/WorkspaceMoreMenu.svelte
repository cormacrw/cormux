<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import { shellDialogs } from '$lib/state'
  import { registerPopoverCloser } from '$lib/keyboard/global-shortcuts'
  import Ellipsis from '@lucide/svelte/icons/ellipsis'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import { onMount } from 'svelte'

  let {
    workspaceId,
  }: {
    workspaceId: string
  } = $props()

  let open = $state(false)

  onMount(() => {
    registerPopoverCloser(() => {
      if (!open) return false
      open = false
      return true
    })
    return () => registerPopoverCloser(null)
  })

  function requestTeardown() {
    open = false
    shellDialogs.openTeardown(workspaceId)
  }
</script>

<DropdownMenu.Root bind:open>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <Button
        {...props}
        variant="outline"
        size="icon-sm"
        aria-label="More workspace actions"
        aria-haspopup="menu"
      >
        <Ellipsis class="size-4" aria-hidden="true" />
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content align="end" class="w-56">
    <DropdownMenu.Item disabled>New thread (soon)</DropdownMenu.Item>
    <DropdownMenu.Separator />
    <DropdownMenu.Item variant="destructive" onclick={requestTeardown}>
      <Trash2 class="size-4" aria-hidden="true" />
      Teardown worktree…
    </DropdownMenu.Item>
  </DropdownMenu.Content>
</DropdownMenu.Root>
