<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import PlaceSwitcher from '$lib/components/shell/PlaceSwitcher.svelte'
  import { trackKeyboardInput } from '$lib/keyboard/focus-home'
  import { switcher } from '$lib/state/switcher.svelte'
  import { currentPlace, handleSwitcherKeyup } from '$lib/switcher/navigate'
  import { ModeWatcher } from 'mode-watcher'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import AppShell from '$lib/components/shell/AppShell.svelte'
  import Backdrop from '$lib/background/Backdrop.svelte'
  import CommandPalette from '$lib/components/shell/CommandPalette.svelte'
  import { bindNativeMenu } from '$lib/components/shell/menu'
  import { handleGlobalKeydown } from '$lib/keyboard/global-shortcuts'
  import { installOverlayFailsafe } from '$lib/overlay-failsafe'
  import { fetchSnapshot, listenForStateChanges } from '$lib/ipc'
  import {
    app,
    clickup,
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
  import SprintView from '$lib/components/sprint/SprintView.svelte'
  import WeekView from '$lib/components/week/WeekView.svelte'
  import Repo from './routes/Repo.svelte'
  import { bindFeedbackEvents } from '$lib/feedback/wire-feedback'
  import { bindWorkspaceAppControls } from '$lib/workspace/wire-workspace-app'
  import { bindGitWorkspaceControls } from '$lib/workspace/wire-git-workspace'
  import { setHarnessWindowFocused } from '$lib/feedback/supervision'
  import { openNotifiedTarget } from '$lib/feedback/os-notifications'
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
      // The window starts hidden (tauri.conf.json) and appears once the first
      // snapshot has painted, so it never shows half-drawn. The core shows it
      // anyway after a few seconds, should this never run.
      void tick().then(() =>
        requestAnimationFrame(() =>
          requestAnimationFrame(() => {
            const win = getCurrentWindow()
            void win
              .show()
              .then(() => win.setFocus())
              .catch(() => {})
          }),
        ),
      )

      const unlistenMenu = await bindNativeMenu()
      const unlistenFeedback = await bindFeedbackEvents()
      const unlistenWorkspaceApp = bindWorkspaceAppControls()
      const unlistenGitWorkspace = bindGitWorkspaceControls()
      const win = getCurrentWindow()
      const unlistenFocus = await win.onFocusChanged(({ payload: focused }) => {
        setHarnessWindowFocused(focused)
        if (focused) {
          openNotifiedTarget()
          void clickup.refresh()
        }
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

  onMount(() => installOverlayFailsafe())

  // Teammates move tasks in ClickUp too, so the sprint refreshes every couple of minutes.
  $effect(() => {
    if (!clickup.ready) return
    const id = window.setInterval(() => void clickup.refresh(), 120_000)
    return () => window.clearInterval(id)
  })

  onMount(() => {
    const cancelSwitcher = () => switcher.cancel()
    const stopTracking = trackKeyboardInput(window)
    window.addEventListener('keydown', handleGlobalKeydown)
    window.addEventListener('keyup', handleSwitcherKeyup)
    // Ctrl can be let go in another app; don't jump somewhere on return.
    window.addEventListener('blur', cancelSwitcher)
    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown)
      window.removeEventListener('keyup', handleSwitcherKeyup)
      window.removeEventListener('blur', cancelSwitcher)
      stopTracking()
    }
  })

  // Ctrl+Tab goes back through these, most recent first.
  $effect(() => {
    const place = currentPlace()
    if (place) untrack(() => switcher.visit(place))
  })

  $effect(() => {
    if (app.view === 'workspace' && app.workspaceId && app.threadId)
      switcher.rememberThread(app.workspaceId, app.threadId)
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
<!-- What the glass sits on: the theme's gradient and the wordmark. -->
<Backdrop />
<PlaceSwitcher />

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
    {:else if app.view === 'sprint'}
      <SprintView />
    {:else if app.view === 'week'}
      <WeekView />
    {:else if app.view === 'repo'}
      <Repo />
    {:else}
      <Homebase />
    {/if}
  </AppShell>
</Tooltip.Provider>
