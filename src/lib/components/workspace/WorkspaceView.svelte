<script lang="ts">
  import { onMount, untrack } from 'svelte'
  import {
    app,
    findings,
    plans,
    stacks,
    threads,
    workspaceRecords,
    workspaceUi,
    workspaces,
  } from '$lib/state'
  import { bindWorkspaceDiffSubscription } from '$lib/state/workspace-diff.svelte'
  import RenameWorkspaceDialog from './RenameWorkspaceDialog.svelte'
  import { isWorkspaceProvisioning } from '$lib/workspace/provisioning'
  import { startStreamingSpike } from '$lib/ipc'
  import ProvisioningFailureBanner from './ProvisioningFailureBanner.svelte'
  import WorkspaceHeader from './WorkspaceHeader.svelte'
  import WorkspaceOutputPanel from './WorkspaceOutputPanel.svelte'
  import ThreadTabBar from './ThreadTabBar.svelte'
  import WorkspaceFindingsPanel from './WorkspaceFindingsPanel.svelte'
  import PlanPanel from './PlanPanel.svelte'
  import ThreadPanel from './thread/ThreadPanel.svelte'
  import ChangesPanel from './changes/ChangesPanel.svelte'

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

  // Opening a workspace on a thread leaves focus to its composer, ready to type; other
  // tabs focus the title.
  $effect(() => {
    if (app.focusTarget !== 'workspace') return
    void app.focusGeneration
    if (untrack(() => workspaceUi.activeTab) !== 'thread')
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
  const workspaceRepoId = $derived(
    workspace ? (workspaceRecords.getRecord(workspace.id)?.repoId ?? '') : '',
  )
  const outputTabActive = $derived(workspaceUi.activeTab === 'output')
  const findingsTabActive = $derived(workspaceUi.activeTab === 'findings')
  const planTabActive = $derived(workspaceUi.activeTab === 'plan')
  const changesTabActive = $derived(workspaceUi.activeTab === 'changes')
  // Git and Output slide over the panel underneath, which stays mounted so
  // closing them is instant and the conversation keeps its scroll.
  const baseTab = $derived(workspaceUi.baseTab)
  const slideOut = $derived(workspaceUi.slideOut)
  // Mounted on first open, then kept so reopening doesn't rebuild the diff or log.
  let changesMounted = $state(false)
  let outputMounted = $state(false)
  $effect(() => {
    if (changesTabActive) changesMounted = true
    if (outputTabActive) outputMounted = true
  })
  $effect(() => {
    void workspaceId
    untrack(() => {
      changesMounted = changesTabActive
      outputMounted = outputTabActive
    })
  })

  // Esc slides Git or Output away from anywhere in the workspace, unless a dialog,
  // menu or popover is open and wants the key for itself.
  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented || !slideOut) return
    if (
      document.querySelector(
        '[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"][data-state="open"]',
      )
    )
      return
    event.preventDefault()
    workspaceUi.closeSlideOut()
  }

  function onSlideOutKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape' || event.defaultPrevented) return
    event.preventDefault()
    workspaceUi.closeSlideOut()
  }

  const threadPanelLabelId = $derived(
    activeThread ? `thread-tab-${activeThread.id}` : undefined,
  )

  // Snapshots rebuild the workspace object; resubscribing on each one would clear the diff.
  const workspaceId = $derived(workspace?.id)

  $effect(() => {
    if (!workspaceId || isLoadSpike) return
    return bindWorkspaceDiffSubscription(workspaceId)
  })

  const workspaceBranch = $derived(workspace?.branch)

  // Opening Changes again picks up commits made since the last look.
  $effect(() => {
    if (!workspaceId || isLoadSpike) return
    void workspaceBranch
    void changesTabActive
    untrack(() => void stacks.load(workspaceId))
  })

  $effect(() => {
    void workspaceUi.activeTab
    void workspaceUi.findingsFocusPending
    queueMicrotask(() => {
      if (
        findingsTabActive &&
        findingsHeading &&
        workspaceUi.findingsFocusPending
      ) {
        findingsHeading.focus({ preventScroll: true })
        workspaceUi.findingsFocusPending = false
      }
    })
  })

  $effect(() => {
    if (!workspace?.id || !findingsTabActive) return
    if (
      workspaceUi.activeTab === 'findings' &&
      !findings.hasReviewFindings(workspace.id)
    ) {
      workspaceUi.openTab('thread')
    }
  })

  // A plan an agent hands over in this workspace opens in the Plan tab, ready to review.
  $effect(() => {
    const plan = plans.arrived
    if (!plan || !workspaceId) return
    untrack(() => {
      if (threads.getById(plan.threadId)?.workspaceId !== workspaceId) return
      plans.arrived = null
      workspaceUi.openPlan(plan.threadId)
    })
  })

  $effect(() => {
    if (!planTabActive || !workspaceId) return
    const ids = workspaceThreads.map((thread) => thread.id)
    if (!plans.visibleFor(workspaceId, ids).length)
      workspaceUi.openTab('thread')
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

<svelte:window onkeydown={onWindowKeydown} />

<section class="flex min-h-0 flex-1 flex-col">
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
      <div class="relative flex min-h-0 flex-1 flex-col overflow-hidden">
        <div class="flex min-h-0 flex-1 flex-col" inert={slideOut != null}>
          {#if baseTab === 'thread'}
            <ThreadPanel
              {workspace}
              thread={activeThread}
              panelLabelId={threadPanelLabelId}
            />
          {:else if baseTab === 'plan'}
            <PlanPanel workspaceId={workspace.id} />
          {:else if baseTab === 'findings'}
            <WorkspaceFindingsPanel
              workspaceId={workspace.id}
              bind:headingRef={findingsHeading}
            />
          {/if}
        </div>

        <!-- Clicking the dimmed conversation slides Git or Output away. -->
        <button
          type="button"
          tabindex="-1"
          aria-hidden="true"
          class="slide-scrim absolute inset-0 z-20 cursor-default bg-black/10 dark:bg-black/25"
          data-open={slideOut != null}
          onclick={() => workspaceUi.closeSlideOut()}
        ></button>

        {#if changesMounted}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="slide-out glass-pane absolute inset-y-0 right-0 z-30 flex w-[min(960px,94%)] flex-col overflow-hidden rounded-tl-[18px]"
            data-open={changesTabActive}
            data-od-id="git-slide-out"
            inert={!changesTabActive}
            onkeydown={onSlideOutKeydown}
          >
            <ChangesPanel
              {workspace}
              thread={activeThread}
              repoId={workspaceRepoId}
              {provisioning}
            />
          </div>
        {/if}
        {#if outputMounted}
          <div
            id="output-panel"
            role="tabpanel"
            aria-labelledby="thread-tab-output"
            tabindex="-1"
            class="slide-out glass-pane absolute inset-y-0 right-0 z-30 flex w-[min(720px,94%)] flex-col overflow-hidden rounded-tl-[18px]"
            data-open={outputTabActive}
            data-od-id="output-slide-out"
            inert={!outputTabActive}
            onkeydown={onSlideOutKeydown}
          >
            <WorkspaceOutputPanel
              workspaceId={workspace.id}
              repoId={workspaceRepoId}
              {provisioning}
            />
          </div>
        {/if}
      </div>
    {/if}
  {:else}
    <p class="text-sm text-muted-foreground">Workspace not found.</p>
  {/if}
</section>
