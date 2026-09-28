<script lang="ts">
  import { onMount } from 'svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import { app, threads, workspaceUi, workspaces } from '$lib/state'
  import { isWorkspaceProvisioning } from '$lib/workspace/provisioning'
  import { startStreamingSpike } from '$lib/ipc'
  import ProvisioningFailureBanner from './ProvisioningFailureBanner.svelte'
  import WorkspaceHeader from './WorkspaceHeader.svelte'
  import WorkspaceOutputPanel from './WorkspaceOutputPanel.svelte'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'

  const workspace = $derived(
    app.workspaceId ? workspaces.getById(app.workspaceId) : undefined,
  )
  const workspaceThreads = $derived(
    app.workspaceId ? threads.forWorkspace(app.workspaceId) : [],
  )
  const activeThread = $derived(
    workspaceThreads.find((thread) => thread.id === app.threadId) ??
      workspaceThreads[0],
  )

  let workspaceTitle: HTMLHeadingElement | undefined = $state()

  $effect(() => {
    if (app.focusTarget !== 'workspace') return
    void app.focusGeneration
    workspaceTitle?.focus()
  })

  const isLoadSpike = $derived(app.workspaceId === 'spike-load')
  const diffLines = Array.from(
    { length: 5000 },
    (_, i) => `${String(i + 1).padStart(4, ' ')}  const row = ${i}`,
  )

  let agentText = $state('')
  let ptyLines = $state<string[]>([])
  let fps = $state(0)
  let frames = 0
  let lastFpsAt = 0

  const provisioning = $derived(
    workspace ? isWorkspaceProvisioning(workspace.lifecycle) : false,
  )
  const outputTabActive = $derived(workspaceUi.activeTab === 'output')

  onMount(() => {
    if (!isLoadSpike) return

    let raf = 0
    lastFpsAt = performance.now()
    const tick = (now: number) => {
      frames += 1
      if (now - lastFpsAt >= 1000) {
        fps = frames
        frames = 0
        lastFpsAt = now
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)

    void startStreamingSpike(
      (chunk) => {
        agentText += chunk.text
      },
      (chunk) => {
        ptyLines = [...ptyLines.slice(-499), chunk.line]
      },
    )

    return () => cancelAnimationFrame(raf)
  })
</script>

<section class="flex flex-1 flex-col gap-4 p-6">
  {#if isLoadSpike}
    <header>
      <h1
        bind:this={workspaceTitle}
        tabindex="-1"
        class="text-xl font-semibold tracking-tight outline-none"
      >
        {workspace?.name ?? app.workspaceId ?? 'Workspace'}
      </h1>
      <p class="text-sm text-muted-foreground">
        {fps} fps · {ptyLines.length} log lines · 5000-line diff
      </p>
    </header>
    <div class="grid min-h-0 flex-1 grid-cols-3 gap-3 text-xs">
      <pre
        class="overflow-auto rounded-md border border-border bg-muted/30 p-3 font-mono">{agentText}</pre>
      <pre
        class="overflow-auto rounded-md border border-border bg-muted/30 p-3 font-mono">{ptyLines.join(
          '\n',
        )}</pre>
      <pre
        class="overflow-auto rounded-md border border-border bg-muted/30 p-3 font-mono">{diffLines.join(
          '\n',
        )}</pre>
    </div>
  {:else if workspace}
    <WorkspaceHeader bind:titleRef={workspaceTitle} {workspace} />

    {#if workspace.lifecycle === 'provisioningFailed'}
      <ProvisioningFailureBanner {workspace} />
    {/if}

    <div
      class="flex flex-wrap items-center gap-2 border-b border-border/60 pb-2"
      role="tablist"
      aria-label="Workspace panels"
    >
      <Button
        role="tab"
        variant={workspaceUi.activeTab === 'thread' ? 'secondary' : 'ghost'}
        size="sm"
        aria-selected={workspaceUi.activeTab === 'thread'}
        onclick={() => workspaceUi.openTab('thread')}
      >
        Thread
      </Button>
      <Button
        role="tab"
        variant={outputTabActive ? 'secondary' : 'ghost'}
        size="sm"
        aria-selected={outputTabActive}
        onclick={() => workspaceUi.openTab('output')}
        class="gap-2"
      >
        Output
        {#if provisioning}
          <LoaderCircle class="size-3.5 animate-spin" aria-hidden="true" />
        {/if}
      </Button>
    </div>

    {#if workspaceUi.activeTab === 'thread'}
      <div class="grid min-h-0 flex-1 gap-3">
        {#if activeThread}
          <div
            class="flex items-center justify-between gap-3 rounded-md border border-border/70 bg-muted/20 px-3 py-2 text-sm"
            aria-live="polite"
          >
            <div class="min-w-0">
              <p class="font-medium">{activeThread.activity}</p>
              <p class="truncate text-xs text-muted-foreground">
                {workspace.activityText}
              </p>
            </div>
            {#if activeThread.status === 'provisioning'}
              <Badge variant="outline" class="shrink-0 gap-1">
                <LoaderCircle class="size-3 animate-spin" aria-hidden="true" />
                Live
              </Badge>
            {/if}
          </div>
        {/if}
        <p class="text-sm text-muted-foreground">
          {workspaceThreads.length} thread{workspaceThreads.length === 1
            ? ''
            : 's'} · conversation UI lands in COR-11
        </p>
      </div>
    {:else if outputTabActive}
      <WorkspaceOutputPanel workspaceId={workspace.id} {provisioning} />
    {/if}
  {:else}
    <p class="text-sm text-muted-foreground">Workspace not found.</p>
  {/if}
</section>
