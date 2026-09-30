<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { Kbd } from '$lib/components/ui/kbd'
  import CommandIcon from '@lucide/svelte/icons/command'
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

<div
  role="group"
  aria-label="App"
  data-od-id="ws-run-controls"
  class="flex shrink-0 items-center gap-2"
>
  {#if appStatus === 'stopped' || appStatus === 'crashed'}
    <Button
      variant="secondary"
      size="xl"
      disabled={appStatus === 'stopped' && runBlocked}
      aria-label={appStatus === 'crashed' ? 'Restart app' : 'Run app'}
      aria-keyshortcuts="Meta+R"
      data-ws-focus={appStatus === 'crashed' ? 'restart' : 'run'}
      data-od-id={appStatus === 'crashed' ? 'ws-restart' : 'ws-run'}
      onclick={() => run(appStatus === 'crashed' ? 'restart' : 'run')}
    >
      {#if appStatus === 'crashed'}
        <RotateCw class="size-4" aria-hidden="true" />
        Restart
      {:else}
        <Play class="size-4" aria-hidden="true" />
        Run
      {/if}
      <Kbd class="gap-0.5" aria-hidden="true"><CommandIcon />R</Kbd>
    </Button>
  {:else}
    <Button
      variant="secondary"
      size="xl"
      aria-label="Restart app"
      aria-keyshortcuts="Meta+R"
      disabled={appStatus === 'starting'}
      data-ws-focus="restart"
      data-od-id="ws-restart"
      onclick={() => run('restart')}
    >
      <RotateCw class="size-4" aria-hidden="true" />
      Restart
      <Kbd class="gap-0.5" aria-hidden="true"><CommandIcon />R</Kbd>
    </Button>
    <Button
      variant="secondary"
      size="xl"
      aria-label="Stop app"
      aria-keyshortcuts="Meta+."
      class="hover:text-destructive"
      data-ws-focus="stop"
      data-od-id="ws-stop"
      onclick={() => run('stop')}
    >
      <Square class="size-4" aria-hidden="true" />
      Stop
      <Kbd class="gap-0.5" aria-hidden="true"><CommandIcon />.</Kbd>
    </Button>
  {/if}
</div>
