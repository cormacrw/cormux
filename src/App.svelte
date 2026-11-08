<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import PlaceSwitcher from '$lib/switcher/PlaceSwitcher.svelte'
  import { trackKeyboardInput } from '$lib/keyboard/focus-home'
  import { switcher } from '$lib/switcher/store.svelte'
  import { currentPlace, handleSwitcherKeyup } from '$lib/switcher/navigate'
  import { ModeWatcher } from 'mode-watcher'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import AppShell from '$lib/components/shell/AppShell.svelte'
  import Backdrop from '$lib/background/Backdrop.svelte'
  import CommandPalette from '$lib/command-palette/CommandPalette.svelte'
  import { bindNativeMenu } from '$lib/components/shell/menu'
  import { handleGlobalKeydown } from '$lib/components/shell/global-shortcuts'
  import { installOverlayFailsafe } from '$lib/overlay-failsafe'
  import { events } from '$lib/ipc'
  import { listenForStateChanges, refreshSnapshot } from '$lib/core/sync'
  import { nav, navigate } from '$lib/core/nav.svelte'
  import { NEW_THREAD, routeIsLive, windowTitle } from '$lib/core/route'
  import { appName } from '$lib/build-mode'
  import {
    memory,
    repos,
    scratches,
    shellDialogs,
    workspaces,
  } from '$lib/state'
  import { threadTimeline } from '$lib/thread/timeline-store.svelte'
  import { clickup } from '$lib/clickup/store.svelte'
  import { settings } from '$lib/settings/store.svelte'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import ToastsRegion from '$lib/components/feedback/ToastsRegion.svelte'
  import NewThreadView from '$lib/components/new-thread/NewThreadView.svelte'
  import WorkspaceView from '$lib/components/workspace/WorkspaceView.svelte'
  import LazyView from '$lib/components/shell/LazyView.svelte'
  import {
    SHELL_DIALOGS,
    loadRepo,
    loadScratch,
    loadSettings,
    loadSprint,
    preloadLazyViews,
  } from '$lib/components/shell/lazy-views'
  import { bindFeedbackEvents } from '$lib/feedback/wire-feedback'
  import { setHarnessWindowFocused } from '$lib/feedback/supervision'
  import { openNotifiedTarget } from '$lib/feedback/os-notifications'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import type { ShellDialogId } from '$lib/state/shell-dialogs.svelte'

  const shellDialogIds = Object.keys(SHELL_DIALOGS) as ShellDialogId[]

  /** How long the first paint waits for the web fonts before showing the window anyway. */
  const FONT_WAIT_MS = 300

  onMount(() => {
    let unlisten: (() => void) | undefined

    void (async () => {
      try {
        await refreshSnapshot()
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
      const win = getCurrentWindow()
      const unlistenFocus = await win.onFocusChanged(({ payload: focused }) => {
        setHarnessWindowFocused(focused)
        if (focused) {
          openNotifiedTarget()
          void clickup.refresh()
        }
      })
      setHarnessWindowFocused(await win.isFocused())
      const unlistenState = await listenForStateChanges()
      const unlistenMemory = await events.memorySampled.listen(({ payload }) =>
        memory.hydrate(payload.sample),
      )

      unlisten = () => {
        unlistenMenu()
        unlistenFeedback()
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

  onMount(() =>
    threadTimeline.onReadError((error) =>
      toastCoreError(error, 'Could not read this thread'),
    ),
  )

  // A workspace torn down, a scratch ended, a repo removed or ClickUp disconnected takes
  // its page with it.
  $effect(() => {
    const live = routeIsLive(nav.route, {
      workspace: (id) => !!workspaces.getById(id),
      scratch: (id) => !!scratches.getById(id),
      repo: (id) => !!repos.getById(id),
      sprint: clickup.configured,
    })
    if (!live) untrack(() => navigate(NEW_THREAD))
  })

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
    if (nav.view === 'workspace' && nav.workspaceId && nav.threadId)
      switcher.rememberThread(nav.workspaceId, nav.threadId)
  })

  $effect(() => {
    document.documentElement.classList.toggle(
      'reduce-motion',
      settings.reduceMotion,
    )
  })

  const openName = $derived.by(() => {
    const route = nav.route
    if (route.view === 'workspace')
      return workspaces.getById(route.workspaceId)?.name
    if (route.view === 'scratch')
      return scratches.getById(route.scratchId)?.title
    if (route.view === 'repo') return repos.getById(route.repoId)?.name
    return undefined
  })

  $effect(() => {
    const title = windowTitle(nav.route, openName, appName)
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
  {#each shellDialogIds as id (id)}
    <LazyView load={SHELL_DIALOGS[id]} when={shellDialogs.isOpen(id)} />
  {/each}
  <CommandPalette />
  <AppShell>
    {#if nav.view === 'settings'}
      <LazyView load={loadSettings} />
    {:else if nav.view === 'workspace'}
      <WorkspaceView />
    {:else if nav.view === 'scratch'}
      <LazyView load={loadScratch} />
    {:else if nav.view === 'sprint'}
      <LazyView load={loadSprint} />
    {:else if nav.view === 'repo'}
      <LazyView load={loadRepo} />
    {:else}
      <NewThreadView />
    {/if}
  </AppShell>
</Tooltip.Provider>
