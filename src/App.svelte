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
  import { events, fetchSnapshot, listenForStateChanges } from '$lib/ipc'
  import {
    app,
    clickup,
    hydrateFromSnapshot,
    memory,
    patchFromEvent,
    resyncAfterGap,
    settings,
    shellDialogs,
  } from '$lib/state'
  import ToastsRegion from '$lib/components/feedback/ToastsRegion.svelte'
  import NewThreadView from '$lib/components/new-thread/NewThreadView.svelte'
  import CreatePrDialog from '$lib/components/workspace/CreatePrDialog.svelte'
  import LazyView from '$lib/components/shell/LazyView.svelte'
  import {
    loadEndScratchDialog,
    loadRepo,
    loadScratch,
    loadSettings,
    loadSprint,
    loadSubmitReviewDialog,
    loadTeardownDialog,
    preloadLazyViews,
  } from '$lib/components/shell/lazy-views'
  import { bindFeedbackEvents } from '$lib/feedback/wire-feedback'
  import { bindWorkspaceAppControls } from '$lib/workspace/wire-workspace-app'
  import { bindGitWorkspaceControls } from '$lib/workspace/wire-git-workspace'
  import { setHarnessWindowFocused } from '$lib/feedback/supervision'
  import { openNotifiedTarget } from '$lib/feedback/os-notifications'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import Workspace from './routes/Workspace.svelte'

  /** How long the first paint waits for the web fonts before showing the window anyway. */
  const FONT_WAIT_MS = 300

  onMount(() => {
    let unlisten: (() => void) | undefined

    void (async () => {
      try {
        hydrateFromSnapshot(await fetchSnapshot())
      } catch (error) {
        console.warn('snapshot unavailable, using empty stores', error)
      }
      // The window starts hidden (tauri.conf.json) and appears once the first
      // snapshot has painted in the app's own fonts, so it never shows half-drawn
      // or swaps typefaces as it opens. The core shows it anyway after a few
      // seconds, should this never run.
      void Promise.race([
        tick().then(() => document.fonts.ready),
        new Promise((resolve) => window.setTimeout(resolve, FONT_WAIT_MS)),
      ]).then(() =>
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

      // Off the first screen, fetched once the main thread has nothing better to do.
      // WebKit has no requestIdleCallback, so a timer stands in there.
      const idle = window.requestIdleCallback?.bind(window)
      if (idle) idle(preloadLazyViews, { timeout: 3000 })
      else window.setTimeout(preloadLazyViews, 1000)

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
        onEvent: patchFromEvent,
        onGap: () => void resyncAfterGap(),
      })
      const unlistenMemory = await events.memorySampled.listen(({ payload }) =>
        memory.hydrate(payload.sample),
      )

      unlisten = () => {
        unlistenMenu()
        unlistenFeedback()
        unlistenWorkspaceApp()
        unlistenGitWorkspace()
        unlistenFocus()
        unlistenState()
        unlistenMemory()
      }
    })()

    return () => {
      unlisten?.()
    }
  })

  onMount(() => installOverlayFailsafe())

  // Teammates move tasks in ClickUp too, so the sprint refreshes every couple of minutes.
  // Regaining focus refreshes it anyway, so a window in the background can skip its turns.
  $effect(() => {
    if (!clickup.ready) return
    const id = window.setInterval(() => {
      if (document.hasFocus()) void clickup.refresh()
    }, 120_000)
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
  <LazyView
    load={loadTeardownDialog}
    when={shellDialogs.teardownWorkspaceId != null}
  />
  <CreatePrDialog />
  <LazyView
    load={loadSubmitReviewDialog}
    when={shellDialogs.submitReviewWorkspaceId != null}
  />
  <LazyView
    load={loadEndScratchDialog}
    when={shellDialogs.endScratch != null}
  />
  <CommandPalette />
  <AppShell>
    {#if app.view === 'settings'}
      <LazyView load={loadSettings} />
    {:else if app.view === 'workspace'}
      <Workspace />
    {:else if app.view === 'scratch'}
      <LazyView load={loadScratch} />
    {:else if app.view === 'sprint'}
      <LazyView load={loadSprint} />
    {:else if app.view === 'repo'}
      <LazyView load={loadRepo} />
    {:else}
      <NewThreadView />
    {/if}
  </AppShell>
</Tooltip.Provider>
