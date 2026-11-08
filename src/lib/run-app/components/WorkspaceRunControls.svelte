<script lang="ts">
  import { tick } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { shortcutAria, shortcutLabel } from '$lib/keyboard/shortcuts'
  import { appControls, controlApp } from '$lib/run-app/actions'
  import type { WorkspaceAppControlAction } from '$lib/ipc/bindings'
  import Play from '@lucide/svelte/icons/play'
  import RotateCw from '@lucide/svelte/icons/rotate-cw'
  import Square from '@lucide/svelte/icons/square'

  let { workspaceId }: { workspaceId: string } = $props()

  const controls = $derived(appControls(workspaceId))
  const up = $derived(controls.start == null)

  let group = $state<HTMLElement>()
  // Run and Stop swap places, so a click from the keyboard would leave focus on nothing.
  let refocus = false

  function press(action: WorkspaceAppControlAction) {
    refocus = !!group?.contains(document.activeElement)
    void controlApp(workspaceId, action)
  }

  $effect(() => {
    void up
    if (!refocus) return
    refocus = false
    void tick().then(() => {
      // Only if focus went down with the button, never away from where the user has moved on to.
      const active = document.activeElement
      if (active && active !== document.body) return
      group?.querySelector('button')?.focus()
    })
  })
</script>

<div
  bind:this={group}
  role="group"
  aria-label="App"
  data-od-id="ws-run-controls"
  class="flex shrink-0 items-center"
>
  {#if controls.start}
    {@const { action, enabled } = controls.start}
    <!-- A crashed app runs again from scratch, so it's Run either way. -->
    <Button
      variant="secondary"
      size="icon"
      disabled={!enabled}
      aria-label="Run app"
      aria-keyshortcuts={shortcutAria('runApp')}
      title="Run app ({shortcutLabel('runApp')})"
      data-od-id="ws-run"
      onclick={() => press(action)}
    >
      <Play class="size-4" aria-hidden="true" />
    </Button>
  {:else}
    <Button
      variant="secondary"
      size="icon"
      aria-label="Stop app"
      aria-keyshortcuts={shortcutAria('stopApp')}
      title="Stop app ({shortcutLabel('stopApp')})"
      class="rounded-r-none hover:text-destructive"
      data-od-id="ws-stop"
      onclick={() => press('stop')}
    >
      <Square class="size-3.5" aria-hidden="true" />
    </Button>
    <span class="h-5 w-px bg-border" aria-hidden="true"></span>
    <Button
      variant="secondary"
      size="icon"
      aria-label="Restart app"
      title="Restart app"
      class="rounded-l-none"
      disabled={!controls.canRestart}
      data-od-id="ws-restart"
      onclick={() => press('restart')}
    >
      <RotateCw class="size-4" aria-hidden="true" />
    </Button>
  {/if}
</div>
