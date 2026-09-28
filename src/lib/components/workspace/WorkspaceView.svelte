<script lang="ts">
  import { onMount } from 'svelte'
  import { app, threads, workspaceUi, workspaces } from '$lib/state'
  import { bindWorkspaceDiffSubscription } from '$lib/state/workspace-diff.svelte'
  import RenameWorkspaceDialog from './RenameWorkspaceDialog.svelte'
  import { isWorkspaceProvisioning } from '$lib/workspace/provisioning'
  import { startStreamingSpike } from '$lib/ipc'
  import ProvisioningFailureBanner from './ProvisioningFailureBanner.svelte'
  import WorkspaceHeader from './WorkspaceHeader.svelte'
  import WorkspaceOutputPanel from './WorkspaceOutputPanel.svelte'
  import ThreadTabBar from './ThreadTabBar.svelte'
  import WorkspaceFindingsPanel from './WorkspaceFindingsPanel.svelte'
  import ThreadPanel from './thread/ThreadPanel.svelte'
  import WorkspaceChangesLayout from './WorkspaceChangesLayout.svelte'

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
  let renameWorkspaceId = $state<string | null>(null)
  let findingsHeading: HTMLHeadingElement | undefined = $state()

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
  const findingsTabActive = $derived(workspaceUi.activeTab === 'findings')
  const threadTabActive = $derived(workspaceUi.activeTab === 'thread')

  const threadPanelLabelId = $derived(
    activeThread ? `thread-tab-${activeThread.id}` : undefined,
  )

  $effect(() => {
    if (!workspace?.id || isLoadSpike) return
    return bindWorkspaceDiffSubscription(workspace.id)
  })

  $effect(() => {
    void workspaceUi.activeTab
    void app.threadId
    queueMicrotask(() => {
      if (findingsTabActive && findingsHeading) {
        findingsHeading.focus({ preventScroll: true })
      }
    })
  })

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

<section class="flex min-h-0 flex-1 flex-col gap-4 p-6">
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
    <RenameWorkspaceDialog bind:workspaceId={renameWorkspaceId} />
    <WorkspaceHeader
      bind:titleRef={workspaceTitle}
      {workspace}
      onRename={() => {
        renameWorkspaceId = workspace.id
      }}
    />

    {#if workspace.lifecycle === 'provisioningFailed'}
      <ProvisioningFailureBanner {workspace} />
    {/if}

    <ThreadTabBar {workspace} />

    {#if activeThread}
      <WorkspaceChangesLayout {workspace} thread={activeThread}>
        {#if threadTabActive}
          <ThreadPanel
            {workspace}
            thread={activeThread}
            panelLabelId={threadPanelLabelId}
          />
        {:else if findingsTabActive}
          <WorkspaceFindingsPanel
            workspaceId={workspace.id}
            bind:headingRef={findingsHeading}
          />
        {:else if outputTabActive}
          <div
            id="output-panel"
            role="tabpanel"
            aria-labelledby="thread-tab-output"
            class="min-h-0 flex-1"
          >
            <WorkspaceOutputPanel workspaceId={workspace.id} {provisioning} />
          </div>
        {/if}
      </WorkspaceChangesLayout>
    {/if}
  {:else}
    <p class="text-sm text-muted-foreground">Workspace not found.</p>
  {/if}
</section>
