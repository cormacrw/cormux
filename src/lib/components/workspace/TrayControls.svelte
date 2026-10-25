<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { workspaceUi } from '$lib/state'
  import type { WorkspaceSlideOut } from '$lib/state/workspace-ui.svelte'
  import Pin from '@lucide/svelte/icons/pin'
  import PinOff from '@lucide/svelte/icons/pin-off'
  import X from '@lucide/svelte/icons/x'

  // Pin and close for the right-hand trays (Git and Output).
  let {
    tray,
    name,
    shortcut,
  }: { tray: WorkspaceSlideOut; name: string; shortcut: string } = $props()

  const pinned = $derived(workspaceUi.pinnedTray === tray)
  const pinLabel = $derived(
    pinned ? `Unpin ${name}` : `Pin ${name} beside the conversation`,
  )
</script>

<Button
  variant={pinned ? 'secondary' : 'ghost'}
  size="icon-sm"
  class="shrink-0 rounded-full"
  aria-label={pinLabel}
  aria-pressed={pinned}
  title={pinLabel}
  data-od-id="tray-pin"
  onclick={() => workspaceUi.setTrayPinned(tray, !pinned)}
>
  {#if pinned}
    <PinOff class="size-3.5" aria-hidden="true" />
  {:else}
    <Pin class="size-3.5" aria-hidden="true" />
  {/if}
</Button>
<Button
  variant="secondary"
  size="icon-sm"
  class="shrink-0 rounded-full"
  aria-label="Close {name}"
  title="Close {name} ({pinned ? shortcut : `Esc or ${shortcut}`})"
  data-od-id="slide-out-close"
  onclick={() => workspaceUi.closeTray(tray)}
>
  <X class="size-3.5" aria-hidden="true" />
</Button>
