<script lang="ts">
  import { tick } from 'svelte'
  import TrayControls from '$lib/components/tray/TrayControls.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Badge } from '$lib/components/ui/badge'
  import { subscribePty } from '$lib/ipc'
  import type { WorkspaceAppControlAction } from '$lib/ipc/bindings'
  import { repos, workspaceUi } from '$lib/state'
  import { workspaces } from '$lib/state/workspaces.svelte'
  import { appControls, controlApp } from '$lib/run-app/actions'
  import { outputBody } from '$lib/run-app/controls'
  import { appStatusView, isAppUp } from '$lib/run-app/status'
  import { openRepoRunCommand } from '$lib/settings/open'
  import {
    OUTPUT_LINE_CAP,
    appendOutputLines,
    type OutputLine,
  } from '$lib/run-app/output-log'
  import { openExternal } from '$lib/feedback/open-external'
  import ExternalLink from '@lucide/svelte/icons/external-link'
  import Eraser from '@lucide/svelte/icons/eraser'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import Play from '@lucide/svelte/icons/play'
  import RotateCw from '@lucide/svelte/icons/rotate-cw'
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal'
  import Square from '@lucide/svelte/icons/square'
  import WorkspaceOutputLog from './WorkspaceOutputLog.svelte'
  import RunCommandField from './RunCommandField.svelte'

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
  let lines = $state.raw<OutputLine[]>([])
  // Lines wait here for the next frame, or for the tray to open. Chatty dev servers print far
  // faster than anyone can read, so the list only changes once a frame.
  let pending: OutputLine[] = []
  let nextLineId = 0
  let flushFrame: number | undefined
  let logEl: HTMLDivElement | undefined = $state()
  let sectionEl: HTMLElement | undefined = $state()
  let stickToBottom = $state(true)

  const runtime = $derived(workspaces.runtime(workspaceId))
  // A primitive, so status or git updates to `runtime` (or a rebuilt workspace behind the
  // `workspaceId` prop) don't wipe the log; only Clear or another workspace does.
  const logKey = $derived(`${workspaceId}:${runtime.logVersion}`)
  const repo = $derived(repos.getById(repoId))
  const repoCommand = $derived(repos.runCommand(repoId))
  const controls = $derived(appControls(workspaceId))
  const runCommand = $derived(controls.runCommand)
  let commandField: RunCommandField | undefined = $state()

  const app = $derived(appStatusView(runtime, provisioning))

  const body = $derived(
    outputBody({
      provisioning,
      appStatus: runtime.appStatus,
      runCommand,
      hasLines: lines.length > 0,
    }),
  )

  function resetLog() {
    lines = []
    pending = []
  }

  function flushLines() {
    if (flushFrame != null) cancelAnimationFrame(flushFrame)
    flushFrame = undefined
    if (!workspaceUi.outputVisible || pending.length === 0) return
    lines = appendOutputLines(lines, pending)
    pending = []
    if (stickToBottom) {
      void tick().then(() => {
        if (logEl) logEl.scrollTop = logEl.scrollHeight
      })
    }
  }

  $effect(() => {
    void logKey
    resetLog()
    stickToBottom = true
  })

  $effect(() => {
    if (workspaceUi.outputVisible) flushLines()
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
      pending.push({ id: nextLineId++, ...chunk.entry })
      if (pending.length > OUTPUT_LINE_CAP) pending.shift()
      if (workspaceUi.outputVisible) {
        flushFrame ??= requestAnimationFrame(flushLines)
      }
    })
  })

  $effect(() => () => {
    if (flushFrame != null) cancelAnimationFrame(flushFrame)
  })

  function onLogScroll() {
    if (!logEl) return
    const distance = logEl.scrollHeight - logEl.scrollTop - logEl.clientHeight
    stickToBottom = distance <= 24
  }

  function run(action: WorkspaceAppControlAction) {
    void controlApp(workspaceId, action)
    // Clear disables itself, so focus moves on: to the emptied log, or to the panel when
    // nothing is running and the log gives way to that note.
    if (action === 'clear') {
      void tick().then(() => (logEl?.isConnected ? logEl : sectionEl)?.focus())
    }
  }

  function openAppUrl() {
    if (!runtime.port) return
    void openExternal(
      `${runtime.https ? 'https' : 'http'}://localhost:${runtime.port}/`,
    )
  }
</script>

<section
  bind:this={sectionEl}
  tabindex="-1"
  class="flex min-h-0 flex-1 flex-col gap-2 outline-none"
  data-od-id="output"
>
  <header
    id="term-head"
    class="flex flex-wrap items-center gap-2 border-b border-border/60 px-3 py-2.5"
    data-od-id="output-head"
  >
    <Badge variant={app.phase === 'running' ? 'default' : 'secondary'}>
      {#if app.busy}
        <LoaderCircle class="mr-1 size-3 animate-spin" aria-hidden="true" />
      {:else if app.phase === 'running'}
        <span class="mr-1 size-2 rounded-full bg-emerald-500" aria-hidden="true"
        ></span>
      {/if}
      {app.word}
    </Badge>

    <RunCommandField
      bind:this={commandField}
      {workspaceId}
      {repoCommand}
      running={isAppUp(runtime.appStatus)}
    />

    {#if app.port}
      <button
        type="button"
        class="inline-flex items-center gap-1 font-mono text-xs text-primary hover:underline"
        onclick={openAppUrl}
      >
        localhost:{app.port}
        <ExternalLink class="size-3" aria-hidden="true" />
      </button>
    {/if}

    <span class="min-w-2 flex-1"></span>

    {#if controls.start?.action === 'run'}
      <Button
        variant="secondary"
        size="sm"
        disabled={!controls.start.enabled}
        aria-label={narrow ? 'Run' : undefined}
        onclick={() => run('run')}
      >
        {#if narrow}<Play class="size-3.5" aria-hidden="true" />{:else}Run{/if}
      </Button>
    {:else if controls.start}
      <Button
        variant="secondary"
        size="sm"
        aria-label={narrow ? 'Restart' : undefined}
        onclick={() => run('restart')}
      >
        <RotateCw class="size-3.5" aria-hidden="true" />
        {#if !narrow}Restart{/if}
      </Button>
    {:else}
      <Button
        variant="ghost"
        size="sm"
        disabled={!controls.canRestart}
        aria-label={narrow ? 'Restart' : undefined}
        onclick={() => run('restart')}
      >
        <RotateCw class="size-3.5" aria-hidden="true" />
        {#if !narrow}Restart{/if}
      </Button>
      <Button
        variant="ghost"
        size="sm"
        class="text-destructive hover:text-destructive"
        aria-label={narrow ? 'Stop' : undefined}
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
      aria-label={narrow ? 'Clear' : undefined}
      onclick={() => run('clear')}
    >
      {#if narrow}<Eraser
          class="size-3.5"
          aria-hidden="true"
        />{:else}Clear{/if}
    </Button>
    <TrayControls tray="output" name="Output" shortcut="toggleOutput" />
  </header>

  {#if body === 'noCommand'}
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
      <div class="flex flex-wrap justify-center gap-2">
        <Button
          variant="secondary"
          size="sm"
          onclick={() => openRepoRunCommand(repoId)}
        >
          <SlidersHorizontal class="size-3.5" aria-hidden="true" />
          Add run command
        </Button>
        <Button
          variant="ghost"
          size="sm"
          data-od-id="run-command-set-here"
          onclick={() => void commandField?.startEditing()}
        >
          Just for this workspace
        </Button>
      </div>
    </div>
  {:else if body === 'idle'}
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
