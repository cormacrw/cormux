<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { runWorkspaceApp } from '$lib/command-palette/actions'
  import { repos, settings, workspaceRecords } from '$lib/state'
  import { workspaceRunCommand } from '$lib/run-app/run-command'
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
  const hasRunCommand = $derived(
    !!workspaceRunCommand(settings.rows, workspaceId, repo?.runCommand),
  )

  const runBlocked = $derived(runDisabled || !hasRunCommand)

  function run(action: 'run' | 'restart' | 'stop') {
    runWorkspaceApp(workspaceId, action)
  }
</script>

<!-- Icon buttons beside ⋯: Play when the app is stopped; Stop and Restart, joined, while it runs. -->
<div
  role="group"
  aria-label="App"
  data-od-id="ws-run-controls"
  class="flex shrink-0 items-center"
>
  {#if appStatus === 'stopped' || appStatus === 'crashed'}
    <!-- A crashed app runs again from scratch, so it's Run either way. -->
    <Button
      variant="secondary"
      size="icon"
      disabled={appStatus === 'stopped' && runBlocked}
      aria-label="Run app"
      aria-keyshortcuts="Meta+R"
      title="Run app (⌘R)"
      data-ws-focus="run"
      data-od-id="ws-run"
      onclick={() => run(appStatus === 'crashed' ? 'restart' : 'run')}
    >
      <Play class="size-4" aria-hidden="true" />
    </Button>
  {:else}
    <Button
      variant="secondary"
      size="icon"
      aria-label="Stop app"
      aria-keyshortcuts="Meta+."
      title="Stop app (⌘.)"
      class="rounded-r-none hover:text-destructive"
      data-ws-focus="stop"
      data-od-id="ws-stop"
      onclick={() => run('stop')}
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
      data-ws-focus="restart"
      data-od-id="ws-restart"
      onclick={() => run('restart')}
    >
      <RotateCw class="size-4" aria-hidden="true" />
    </Button>
  {/if}
</div>
