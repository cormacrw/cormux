<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { shortcutAria, shortcutLabel } from '$lib/keyboard/shortcuts'
  import { appControls, controlApp } from '$lib/run-app/actions'
  import Play from '@lucide/svelte/icons/play'
  import RotateCw from '@lucide/svelte/icons/rotate-cw'
  import Square from '@lucide/svelte/icons/square'

  let { workspaceId }: { workspaceId: string } = $props()

  const controls = $derived(appControls(workspaceId))
  const start = $derived(controls.start)
</script>

<div
  role="group"
  aria-label="App"
  data-od-id="ws-run-controls"
  class="flex shrink-0 items-center"
>
  <!-- One button that is Run or Stop, so focus stays on it as they swap. A crashed app runs
       again from scratch, so it's Run either way. -->
  <Button
    variant="secondary"
    size="icon"
    disabled={start ? !start.enabled : false}
    aria-label={start ? 'Run app' : 'Stop app'}
    aria-keyshortcuts={shortcutAria(start ? 'runApp' : 'stopApp')}
    title={start
      ? `Run app (${shortcutLabel('runApp')})`
      : `Stop app (${shortcutLabel('stopApp')})`}
    class={start ? undefined : 'rounded-r-none hover:text-destructive'}
    data-od-id={start ? 'ws-run' : 'ws-stop'}
    onclick={() => void controlApp(workspaceId, start ? start.action : 'stop')}
  >
    {#if start}
      <Play class="size-4" aria-hidden="true" />
    {:else}
      <Square class="size-3.5" aria-hidden="true" />
    {/if}
  </Button>
  {#if !start}
    <span class="h-5 w-px bg-border" aria-hidden="true"></span>
    <Button
      variant="secondary"
      size="icon"
      aria-label="Restart app"
      title="Restart app"
      class="rounded-l-none"
      disabled={!controls.canRestart}
      data-od-id="ws-restart"
      onclick={() => void controlApp(workspaceId, 'restart')}
    >
      <RotateCw class="size-4" aria-hidden="true" />
    </Button>
  {/if}
</div>
