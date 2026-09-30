<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { Badge } from '$lib/components/ui/badge'
  import {
    openRepoRunCommand,
    runWorkspaceApp,
  } from '$lib/command-palette/actions'
  import { subscribePty } from '$lib/ipc'
  import { repos, workspaceRecords } from '$lib/state'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import ExternalLink from '@lucide/svelte/icons/external-link'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import RotateCw from '@lucide/svelte/icons/rotate-cw'
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal'
  import Square from '@lucide/svelte/icons/square'
  import WorkspaceOutputLog from './WorkspaceOutputLog.svelte'

  let {
    workspaceId,
    repoId,
    provisioning,
  }: {
    workspaceId: string
    repoId: string
    provisioning: boolean
  } = $props()

  let narrow = $state(false)
  let lines = $state<string[]>([])
  let logEl: HTMLDivElement | undefined = $state()
  let stickToBottom = $state(true)

  const runtime = $derived(workspaceRecords.runtime(workspaceId))
  // A primitive, so status or git updates to `runtime` don't wipe the log.
  const logVersion = $derived(runtime.logVersion)
  const repo = $derived(repos.getById(repoId))
  const runCommand = $derived(repo?.runCommand?.trim() ?? '')

  const chipLabel = $derived(
    provisioning
      ? 'Setting up'
      : runtime.appStatus === 'starting'
        ? 'Starting'
        : runtime.appStatus === 'running'
          ? 'Running'
          : runtime.appStatus === 'crashed'
            ? `Crashed${runtime.exitCode != null ? ` (${runtime.exitCode})` : ''}`
            : 'Stopped',
  )

  const showEmptyNoCommand = $derived(
    !provisioning && !runCommand && lines.length === 0,
  )
  const showEmptyIdle = $derived(
    !provisioning && !!runCommand && lines.length === 0,
  )

  $effect(() => {
    void logVersion
    lines = []
    stickToBottom = true
  })

  $effect(() => {
    void workspaceId
    lines = []
    stickToBottom = true
  })

  $effect(() => {
    const media = window.matchMedia('(max-width: 640px)')
    const sync = () => {
      narrow = media.matches
    }
    sync()
    media.addEventListener('change', sync)
    return () => media.removeEventListener('change', sync)
  })

  $effect(() => {
    const id = workspaceId
    return subscribePty(id, (chunk) => {
      if (chunk.workspaceId !== id) return
      lines = [...lines.slice(-499), chunk.line]
      if (stickToBottom) {
        queueMicrotask(() => {
          if (logEl) logEl.scrollTop = logEl.scrollHeight
        })
      }
    })
  })

  function onLogScroll() {
    if (!logEl) return
    const distance = logEl.scrollHeight - logEl.scrollTop - logEl.clientHeight
    stickToBottom = distance <= 24
  }

  function run(action: 'run' | 'restart' | 'stop' | 'clear') {
    if (action === 'clear') {
      runWorkspaceApp(workspaceId, 'clear')
      lines = []
      queueMicrotask(() => logEl?.focus())
      return
    }
    runWorkspaceApp(workspaceId, action)
  }

  function openAppUrl() {
    if (!runtime.port) return
    void openUrl(`http://localhost:${runtime.port}/`)
  }
</script>

<section class="flex min-h-0 flex-1 flex-col gap-2" data-od-id="output">
  <header
    id="term-head"
    class="flex flex-wrap items-center gap-2 border-b border-border/60 px-3 py-2.5"
    data-od-id="output-head"
  >
    <Badge variant={runtime.appStatus === 'running' ? 'default' : 'secondary'}>
      {#if provisioning || runtime.appStatus === 'starting'}
        <LoaderCircle class="mr-1 size-3 animate-spin" aria-hidden="true" />
      {:else if runtime.appStatus === 'running'}
        <span class="mr-1 size-2 rounded-full bg-emerald-500" aria-hidden="true"
        ></span>
      {/if}
      {chipLabel}
    </Badge>

    {#if runCommand}
      <span
        class="max-w-[12rem] truncate font-mono text-xs text-muted-foreground"
        title={runCommand}>{runCommand}</span
      >
    {/if}

    {#if runtime.appStatus === 'running' && runtime.port}
      <button
        type="button"
        class="inline-flex items-center gap-1 font-mono text-xs text-primary hover:underline"
        onclick={openAppUrl}
      >
        localhost:{runtime.port}
        <ExternalLink class="size-3" aria-hidden="true" />
      </button>
    {/if}

    <span class="min-w-2 flex-1"></span>

    {#if runtime.appStatus === 'stopped'}
      <Button
        variant="secondary"
        size="sm"
        disabled={provisioning || !runCommand}
        onclick={() => run('run')}
      >
        {#if !narrow}Run{/if}
      </Button>
    {:else if runtime.appStatus === 'crashed'}
      <Button variant="secondary" size="sm" onclick={() => run('restart')}>
        <RotateCw class="size-3.5" aria-hidden="true" />
        {#if !narrow}Restart{/if}
      </Button>
    {:else}
      <Button
        variant="ghost"
        size="sm"
        disabled={runtime.appStatus === 'starting'}
        onclick={() => run('restart')}
      >
        <RotateCw class="size-3.5" aria-hidden="true" />
        {#if !narrow}Restart{/if}
      </Button>
      <Button
        variant="ghost"
        size="sm"
        class="text-destructive hover:text-destructive"
        onclick={() => run('stop')}
      >
        <Square class="size-3.5" aria-hidden="true" />
        {#if !narrow}Stop{/if}
      </Button>
    {/if}

    <Button
      variant="ghost"
      size="sm"
      disabled={lines.length === 0}
      onclick={() => run('clear')}
    >
      {#if !narrow}Clear{/if}
    </Button>
  </header>

  {#if showEmptyNoCommand}
    <div
      class="flex flex-1 flex-col items-center justify-center gap-2 px-4 py-12 text-center"
    >
      <h2 class="text-sm font-medium">
        No run command for {repo?.name ?? repoId}
      </h2>
      <p class="max-w-md text-sm text-muted-foreground">
        Add the command that starts this project, like pnpm dev. Every workspace
        on {repo?.name ?? repoId} will use it.
      </p>
      <Button
        variant="secondary"
        size="sm"
        onclick={() => openRepoRunCommand(repoId)}
      >
        <SlidersHorizontal class="size-3.5" aria-hidden="true" />
        Add run command
      </Button>
    </div>
  {:else if showEmptyIdle}
    <div
      class="flex flex-1 flex-col items-center justify-center gap-2 px-4 py-12 text-center"
    >
      <h2 class="text-sm font-medium">Nothing running</h2>
      <p class="max-w-md text-sm text-muted-foreground">
        Run starts {runCommand} in this worktree. Its output shows up here.
      </p>
    </div>
  {:else}
    <WorkspaceOutputLog {lines} onScroll={onLogScroll} bind:logEl />
  {/if}
</section>
