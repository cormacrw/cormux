<script lang="ts">
  import { onMount } from 'svelte'
  import { ModeWatcher } from 'mode-watcher'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import AppShell from '$lib/components/shell/AppShell.svelte'
  import CommandPalette from '$lib/components/shell/CommandPalette.svelte'
  import { bindNativeMenu } from '$lib/components/shell/menu'
  import { handleGlobalKeydown } from '$lib/keyboard/global-shortcuts'
  import { fetchSnapshot, listenForStateChanges } from '$lib/ipc'
  import {
    app,
    hydrateFromSnapshot,
    patchFromEvent,
    settings,
  } from '$lib/state'
  import ToastsRegion from '$lib/components/feedback/ToastsRegion.svelte'
  import NewWorkspaceDialog from '$lib/components/new-workspace/NewWorkspaceDialog.svelte'
  import TeardownDialog from '$lib/components/workspace/TeardownDialog.svelte'
  import CreatePrDialog from '$lib/components/workspace/CreatePrDialog.svelte'
  import SubmitReviewDialog from '$lib/components/workspace/SubmitReviewDialog.svelte'
  import NewScratchDialog from '$lib/components/scratch/NewScratchDialog.svelte'
  import EndScratchDialog from '$lib/components/scratch/EndScratchDialog.svelte'
  import ScratchView from '$lib/components/scratch/ScratchView.svelte'
  import TodosView from '$lib/components/todos/TodosView.svelte'
  import { bindFeedbackEvents } from '$lib/feedback/wire-feedback'
  import { bindWorkspaceAppControls } from '$lib/workspace/wire-workspace-app'
  import { bindGitWorkspaceControls } from '$lib/workspace/wire-git-workspace'
  import { setHarnessWindowFocused } from '$lib/feedback/supervision'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import Homebase from './routes/Homebase.svelte'
  import Settings from './routes/Settings.svelte'
  import Workspace from './routes/Workspace.svelte'

  onMount(() => {
    let unlisten: (() => void) | undefined

    void (async () => {
      try {
        hydrateFromSnapshot(await fetchSnapshot())
      } catch (error) {
        console.warn('snapshot unavailable, using empty stores', error)
      }

      const unlistenMenu = await bindNativeMenu()
      const unlistenFeedback = await bindFeedbackEvents()
      const unlistenWorkspaceApp = bindWorkspaceAppControls()
      const unlistenGitWorkspace = bindGitWorkspaceControls()
      const win = getCurrentWindow()
      const unlistenFocus = await win.onFocusChanged(({ payload: focused }) => {
        setHarnessWindowFocused(focused)
      })
      setHarnessWindowFocused(await win.isFocused())
      const unlistenState = await listenForStateChanges({
        lastVersion: () => app.version,
        onEvent: (event) => {
          void patchFromEvent(event)
        },
        onSnapshot: hydrateFromSnapshot,
      })

      unlisten = () => {
        unlistenMenu()
        unlistenFeedback()
        unlistenWorkspaceApp()
        unlistenGitWorkspace()
        unlistenFocus()
        unlistenState()
      }
    })()

    return () => {
      unlisten?.()
    }
  })

  onMount(() => {
    window.addEventListener('keydown', handleGlobalKeydown)
    return () => window.removeEventListener('keydown', handleGlobalKeydown)
  })

  $effect(() => {
    document.documentElement.classList.toggle(
      'reduce-motion',
      settings.reduceMotion,
    )
  })

  $effect(() => {
    const title = app.windowTitle
    void getCurrentWindow()
      .setTitle(title)
      .catch(() => {
        document.title = title
      })
  })
</script>

<ModeWatcher />

<Tooltip.Provider>
  <ToastsRegion />
  <NewWorkspaceDialog />
  <TeardownDialog />
  <CreatePrDialog />
  <SubmitReviewDialog />
  <NewScratchDialog />
  <EndScratchDialog />
  <CommandPalette />
  <AppShell>
    {#if app.view === 'settings'}
      <Settings />
    {:else if app.view === 'workspace'}
      <Workspace />
    {:else if app.view === 'scratch'}
      <ScratchView />
    {:else if app.view === 'todos'}
      <TodosView />
    {:else}
      <Homebase />
    {/if}
  </AppShell>
</Tooltip.Provider>
