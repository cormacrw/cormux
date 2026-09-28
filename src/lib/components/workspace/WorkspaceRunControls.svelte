<script lang="ts">
  import * as InputGroup from '$lib/components/ui/input-group'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import { runWorkspaceApp } from '$lib/command-palette/actions'
  import { repos, workspaceRecords, workspaceUi } from '$lib/state'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import Play from '@lucide/svelte/icons/play'
  import RotateCw from '@lucide/svelte/icons/rotate-cw'
  import Square from '@lucide/svelte/icons/square'
  import Terminal from '@lucide/svelte/icons/terminal'

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
  const outputPressed = $derived(workspaceUi.activeTab === 'output')

  const outputLabel = $derived(
    appStatus === 'running' && runtime.port
      ? `localhost:${runtime.port}`
      : appStatus === 'starting'
        ? 'Starting…'
        : 'Output',
  )

  const outputAria = $derived(
    appStatus === 'running' && runtime.port
      ? `app running on localhost:${runtime.port}`
      : appStatus === 'starting'
        ? 'app starting'
        : 'app stopped',
  )

  const runTitle = $derived(
    runDisabled
      ? 'Available once the worktree is set up'
      : repo?.runCommand
        ? `Run ${repo.runCommand}`
        : `No run command set for ${repo?.name ?? repoId}`,
  )

  function toggleOutput() {
    workspaceUi.toggleOutputTab()
  }

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
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <InputGroup.Button
          {...props}
          variant={outputPressed ? 'secondary' : 'ghost'}
          aria-pressed={outputPressed}
          aria-controls="output-panel"
          aria-label="Output, {outputAria}"
          data-ws-focus="output-toggle"
          data-od-id="ws-output-toggle"
          class="gap-1.5 px-2.5 font-normal"
          onclick={toggleOutput}
        >
          {#if appStatus === 'running'}
            <span
              class="size-2 shrink-0 rounded-full bg-emerald-500"
              aria-hidden="true"
            ></span>
            <span class="font-mono text-xs">{outputLabel}</span>
          {:else if appStatus === 'starting'}
            <LoaderCircle class="size-3.5 animate-spin" aria-hidden="true" />
            <span class="text-xs">{outputLabel}</span>
          {:else}
            <Terminal class="size-3.5" aria-hidden="true" />
            <span class="text-xs">{outputLabel}</span>
          {/if}
        </InputGroup.Button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content>
      {outputPressed ? 'Back to thread' : 'Show output'} (⌃`)
    </Tooltip.Content>
  </Tooltip.Root>

  {#if appStatus === 'stopped'}
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <InputGroup.Button
            {...props}
            disabled={runDisabled}
            aria-label="Run app"
            data-ws-focus="run"
            data-od-id="ws-run"
            class="gap-1 px-2.5"
            onclick={() => run('run')}
          >
            <Play class="size-3.5" aria-hidden="true" />
            <span class="text-xs">Run</span>
          </InputGroup.Button>
        {/snippet}
      </Tooltip.Trigger>
      {#if runDisabled}
        <Tooltip.Content>{runTitle}</Tooltip.Content>
      {:else}
        <Tooltip.Content>{runTitle}</Tooltip.Content>
      {/if}
    </Tooltip.Root>
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
      data-ws-focus="stop"
      data-od-id="ws-stop"
      onclick={() => run('stop')}
    >
      <Square class="size-3.5" aria-hidden="true" />
    </InputGroup.Button>
  {/if}
</InputGroup.Root>
