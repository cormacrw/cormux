<script lang="ts">
  import { tick, untrack } from 'svelte'
  import { threads, workspaceUi, workspaces } from '$lib/state'
  import { findings } from '$lib/findings/store.svelte'
  import { nav } from '$lib/core/nav.svelte'
  import { loadStack } from '$lib/changes/actions'
  import { bindWorkspaceDiffSubscription } from '$lib/changes/diff-store.svelte'
  import RenameWorkspaceDialog from './RenameWorkspaceDialog.svelte'
  import { isWorkspaceProvisioning } from '$lib/state/activity'
  import ProvisioningFailureBanner from './ProvisioningFailureBanner.svelte'
  import WorkspaceHeader from './WorkspaceHeader.svelte'
  import WorkspaceOutputPanel from '$lib/run-app/components/WorkspaceOutputPanel.svelte'
  import WorkspaceFindingsPanel from '$lib/findings/components/WorkspaceFindingsPanel.svelte'
  import ThreadPanel from './thread/ThreadPanel.svelte'
  import { provideMediaHost } from './thread/reply/media-host'
  import ChangesPanel from '$lib/changes/components/ChangesPanel.svelte'
  import { resizeEdge } from '$lib/primitives/slide-out'
  import { focusPrompt } from '$lib/keyboard/focus-home'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'

  const workspace = $derived(
    nav.workspaceId ? workspaces.getById(nav.workspaceId) : undefined,
  )
  const activeThread = $derived(
    nav.workspaceId
      ? threads.shownIn(nav.workspaceId, nav.threadId)
      : undefined,
  )

  let workspaceTitle: HTMLHeadingElement | undefined = $state()
  let sectionEl: HTMLElement | undefined = $state()
  provideMediaHost(() => sectionEl)
  let renameWorkspaceId = $state<string | null>(null)
  let findingsHeading: HTMLHeadingElement | undefined = $state()

  // Opening a workspace on a thread leaves focus to its composer, ready to type; other
  // panels focus the title.
  $effect(() =>
    nav.onArrival(() => {
      if (nav.view !== 'workspace') return
      if (untrack(() => workspaceUi.activeTab) !== 'thread')
        workspaceTitle?.focus()
    }),
  )

  const provisioning = $derived(
    workspace ? isWorkspaceProvisioning(workspace.lifecycle) : false,
  )
  const workspaceRepoId = $derived(workspace?.repoId ?? '')
  const findingsTabActive = $derived(workspaceUi.activeTab === 'findings')
  // A pinned tray sits beside the conversation as a pane, and both stay usable.
  const pinnedTray = $derived(workspaceUi.pinnedTray)
  const gitVisible = $derived(workspaceUi.gitVisible)
  const outputVisible = $derived(workspaceUi.outputVisible)
  // Everything else slides over the conversation, which stays mounted so
  // closing it is instant and the conversation keeps its scroll.
  const slideOut = $derived(workspaceUi.slideOut)
  // Mounted on first open, then kept so reopening doesn't rebuild the diff or log.
  let changesMounted = $state(false)
  let outputMounted = $state(false)
  // A first open mounts the panel closed and opens it a frame later, so it
  // slides in like every later open instead of appearing in place.
  let changesArmed = $state(false)
  let outputArmed = $state(false)
  function nextFrame(run: () => void) {
    requestAnimationFrame(() => requestAnimationFrame(run))
  }
  $effect(() => {
    if (gitVisible && !untrack(() => changesMounted)) {
      changesMounted = true
      nextFrame(() => (changesArmed = true))
    }
    if (outputVisible && !untrack(() => outputMounted)) {
      outputMounted = true
      nextFrame(() => (outputArmed = true))
    }
  })
  // Switching workspaces shows whatever was open straight away.
  $effect(() => {
    void workspaceId
    untrack(() => {
      changesMounted = changesArmed = gitVisible
      outputMounted = outputArmed = outputVisible
      findingsShown = findingsArmed = findingsTabActive
    })
  })
  // Holds its panel while it slides away.
  let findingsShown = $state(false)
  let findingsArmed = $state(false)
  $effect(() => {
    if (!findingsTabActive) {
      findingsArmed = false
      return
    }
    findingsShown = true
    nextFrame(() => (findingsArmed = true))
  })
  const changesOpen = $derived(gitVisible && changesArmed)
  const outputOpen = $derived(outputVisible && outputArmed)
  const PANE_CLASS =
    'tray-pane glass-pane relative my-3 mr-3 flex max-w-[calc(100%-372px)] min-w-[320px] shrink-0 flex-col overflow-hidden rounded-[18px]'
  const SLIDE_CLASS =
    'slide-out glass-pane absolute inset-y-3 right-3 z-30 flex w-[min(var(--tray-w),calc(94%-12px))] flex-col overflow-hidden rounded-[18px]'

  // Esc is a keyboard move, so the composer won't reclaim focus by itself; send it back.
  function dismissSlideOut() {
    workspaceUi.closeSlideOut()
    void tick().then(focusPrompt)
  }

  let paneRow: HTMLDivElement | undefined = $state()
  let changesTray: HTMLDivElement | undefined = $state()
  let outputTray: HTMLDivElement | undefined = $state()

  /** Focus is in the tray pinned beside the conversation. */
  function inPinnedTray() {
    const tray = pinnedTray === 'changes' ? changesTray : outputTray
    return !!pinnedTray && !!tray?.contains(document.activeElement)
  }

  // Esc slides Git, Output or Findings away from anywhere in the workspace; in the pinned
  // tray it goes back to the prompt rather than on to stop the agent.
  $effect(() =>
    keyboardLayers.onEscape('slideOut', () => {
      if (!slideOut && !inPinnedTray()) return false
      dismissSlideOut()
      return true
    }),
  )

  // Snapshots rebuild the workspace object; resubscribing on each one would clear the diff.
  const workspaceId = $derived(workspace?.id)

  $effect(() => {
    if (!workspaceId) return
    return bindWorkspaceDiffSubscription(workspaceId)
  })

  const workspaceBranch = $derived(workspace?.branch)

  // Opening Changes again picks up commits made since the last look.
  $effect(() => {
    if (!workspaceId) return
    void workspaceBranch
    void gitVisible
    untrack(() => void loadStack(workspaceId))
  })

  // Opened on Findings (a toast, the palette, the card in the thread): focus their heading.
  $effect(() => {
    const route = nav.route
    if (route.view !== 'workspace' || route.panel !== 'findings') return
    if (!findingsTabActive || !findingsHeading) return
    const heading = findingsHeading
    untrack(() => nav.take('workspace', 'panel'))
    void tick().then(() => heading.focus({ preventScroll: true }))
  })

  // A workspace whose Findings tab went away (or never had one) leaves it for the thread.
  $effect(() => {
    if (!workspace || !findingsTabActive) return
    if (!findings.tabAvailable(workspace)) workspaceUi.openTab('thread')
  })
</script>

<section bind:this={sectionEl} class="relative flex min-h-0 flex-1 flex-col">
  {#if workspace}
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

    {#if activeThread}
      <div
        bind:this={paneRow}
        class="relative flex min-h-0 flex-1 overflow-hidden"
      >
        <div
          class="workspace-main flex min-h-0 min-w-0 flex-1 flex-col"
          inert={slideOut != null}
        >
          <ThreadPanel {workspace} thread={activeThread} />
        </div>

        <!-- Clicking the dimmed conversation slides Git, Output or Findings away. -->
        <button
          type="button"
          tabindex="-1"
          aria-hidden="true"
          class="slide-scrim absolute inset-0 z-20 cursor-default bg-black/10 dark:bg-black/25"
          data-open={slideOut != null}
          onclick={() => workspaceUi.closeSlideOut()}
        ></button>

        <!-- Dragging the pinned pane's left edge resizes it, leaving the conversation at least 360px. -->
        {#snippet paneEdge()}
          <div
            class="absolute inset-y-0 left-0 z-10 w-1.5 cursor-col-resize"
            title="Drag to resize"
            use:resizeEdge={{
              bounds: () => paneRow?.getBoundingClientRect(),
              min: 320,
              reserve: 372,
              onResize: (width) => workspaceUi.setTrayWidth(width),
            }}
          ></div>
        {/snippet}

        {#if changesMounted}
          <div
            bind:this={changesTray}
            class={pinnedTray === 'changes' ? PANE_CLASS : SLIDE_CLASS}
            style:--tray-w="960px"
            style:width={pinnedTray === 'changes'
              ? `${workspaceUi.trayWidth}px`
              : null}
            data-open={changesOpen}
            data-pinned={pinnedTray === 'changes'}
            data-od-id="git-slide-out"
            data-tray="git"
            inert={!gitVisible}
          >
            {#if pinnedTray === 'changes'}{@render paneEdge()}{/if}
            <ChangesPanel {workspace} repoId={workspaceRepoId} />
          </div>
        {/if}
        {#if outputMounted}
          <div
            bind:this={outputTray}
            id="output-panel"
            role="region"
            aria-label="Output"
            tabindex="-1"
            class={pinnedTray === 'output' ? PANE_CLASS : SLIDE_CLASS}
            style:--tray-w="720px"
            style:width={pinnedTray === 'output'
              ? `${workspaceUi.trayWidth}px`
              : null}
            data-open={outputOpen}
            data-pinned={pinnedTray === 'output'}
            data-od-id="output-slide-out"
            data-tray="output"
            inert={!outputVisible}
          >
            {#if pinnedTray === 'output'}{@render paneEdge()}{/if}
            <WorkspaceOutputPanel
              workspaceId={workspace.id}
              repoId={workspaceRepoId}
              {provisioning}
            />
          </div>
        {/if}
        {#if findingsShown}
          <div
            class={SLIDE_CLASS}
            style:--tray-w="760px"
            data-open={findingsTabActive && findingsArmed}
            data-od-id="findings-slide-out"
            data-tray="findings"
            inert={!findingsTabActive}
          >
            <WorkspaceFindingsPanel
              workspaceId={workspace.id}
              threadId={activeThread.id}
              bind:headingRef={findingsHeading}
            />
          </div>
        {/if}
      </div>
    {/if}
  {:else}
    <p class="text-sm text-muted-foreground">Workspace not found.</p>
  {/if}
</section>
