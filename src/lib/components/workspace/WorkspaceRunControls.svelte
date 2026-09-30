<script lang="ts">
  import * as InputGroup from '$lib/components/ui/input-group'
  import { runWorkspaceApp } from '$lib/command-palette/actions'
  import { repos, workspaceRecords } from '$lib/state'
  import Play from '@lucide/svelte/icons/play'
  import RotateCw from '@lucide/svelte/icons/rotate-cw'
  import Square from '@lucide/svelte/icons/square'

  let {
    workspaceId,
    repoId,
    runDisabled,
  }: {
    workspaceId: string
    repoId: string
    runDisabled: boolean
  } = $props()

  const runtime = $derived(workspaceRecords.runtime(workspaceId))
  const appStatus = $derived(runtime.appStatus)
  const repo = $derived(repos.getById(repoId))
  const hasRunCommand = $derived(!!repo?.runCommand?.trim())

  const runBlocked = $derived(runDisabled || !hasRunCommand)

  function run(action: 'run' | 'restart' | 'stop') {
    runWorkspaceApp(workspaceId, action)
  }
</script>

<InputGroup.Root
  role="group"
  aria-label="App"
  data-od-id="ws-run-controls"
  class="w-auto shrink-0"
>
  {#if appStatus === 'stopped' || appStatus === 'crashed'}
    <InputGroup.Button
      disabled={appStatus === 'stopped' && runBlocked}
      aria-label={appStatus === 'crashed' ? 'Restart app' : 'Run app'}
      data-ws-focus={appStatus === 'crashed' ? 'restart' : 'run'}
      data-od-id={appStatus === 'crashed' ? 'ws-restart' : 'ws-run'}
      class="gap-1 px-2.5"
      onclick={() => run(appStatus === 'crashed' ? 'restart' : 'run')}
    >
      {#if appStatus === 'crashed'}
        <RotateCw class="size-3.5" aria-hidden="true" />
        <span class="text-xs">Restart</span>
      {:else}
        <Play class="size-3.5" aria-hidden="true" />
        <span class="text-xs">Run</span>
      {/if}
    </InputGroup.Button>
  {:else}
    <InputGroup.Button
      variant="ghost"
      aria-label="Restart app"
      disabled={appStatus === 'starting'}
      data-ws-focus="restart"
      data-od-id="ws-restart"
      onclick={() => run('restart')}
    >
      <RotateCw class="size-3.5" aria-hidden="true" />
    </InputGroup.Button>
    <InputGroup.Button
      variant="ghost"
      aria-label="Stop app"
      class="hover:text-destructive"
      data-ws-focus="stop"
      data-od-id="ws-stop"
      onclick={() => run('stop')}
    >
      <Square class="size-3.5" aria-hidden="true" />
    </InputGroup.Button>
  {/if}
</InputGroup.Root>
