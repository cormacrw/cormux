<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import PlaceSwitcher from '$lib/switcher/PlaceSwitcher.svelte'
  import { trackKeyboardInput } from '$lib/keyboard/focus-home'
  import { switcher } from '$lib/switcher/store.svelte'
  import { handleSwitcherKeyup } from '$lib/switcher/navigate'
  import { ModeWatcher } from 'mode-watcher'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import AppShell from '$lib/components/shell/AppShell.svelte'
  import Backdrop from '$lib/background/Backdrop.svelte'
  import CommandPalette from '$lib/command-palette/CommandPalette.svelte'
  import { bindNativeMenu } from '$lib/components/shell/menu'
  import { handleGlobalKeydown } from '$lib/components/shell/global-shortcuts'
  import { installOverlayFailsafe } from '$lib/overlay-failsafe'
  import { listenForStateChanges, refreshSnapshot } from '$lib/core/sync'
  import { nav, navigate } from '$lib/core/nav.svelte'
  import {
    NEW_THREAD,
    routeIsLive,
    windowTitle,
    type RouteLookups,
  } from '$lib/core/route'
  import { appName } from '$lib/build-mode'
  import { repos, scratches, shellDialogs, workspaces } from '$lib/state'
  import { threadTimeline } from '$lib/thread/timeline-store.svelte'
  import { clickup } from '$lib/clickup/store.svelte'
  import { settings } from '$lib/settings/store.svelte'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import ToastsRegion from '$lib/components/feedback/ToastsRegion.svelte'
  import LazyView from '$lib/components/shell/LazyView.svelte'
  import { SHELL_DIALOGS } from '$lib/components/shell/lazy-views'
  import {
    ROUTE_VIEWS,
    preloadLazyViews,
  } from '$lib/components/shell/route-views'
  import { bindFeedbackEvents } from '$lib/feedback/wire-feedback'
  import { installSupervision } from '$lib/feedback/supervision'
  import { installClickupSync } from '$lib/clickup/sync'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import type { ShellDialogId } from '$lib/state/shell-dialogs.svelte'

  const shellDialogIds = Object.keys(SHELL_DIALOGS) as ShellDialogId[]

  /** How long the first paint waits for the web fonts before showing the window anyway. */
  const FONT_WAIT_MS = 300

  onMount(() => {
    const stops: (() => void)[] = []
    let disposed = false
    // Each listener registers after an await; one that lands after unmount stops at once.
    const keep = (stop: () => void) => {
      if (disposed) stop()
      else stops.push(stop)
    }

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

      const win = getCurrentWindow()
      keep(await bindNativeMenu())
      keep(await bindFeedbackEvents())
      keep(await installSupervision(win))
      keep(await installClickupSync(win))
      keep(await listenForStateChanges())
    })()

    return () => {
      disposed = true
      for (const stop of stops) stop()
    }
  })

  onMount(() => installOverlayFailsafe())

  onMount(() =>
    threadTimeline.onReadError((error) =>
      toastCoreError(error, 'Could not read this thread'),
    ),
  )

  const routeLookups: RouteLookups = {
    workspace: (id) => workspaces.getById(id)?.name,
    scratch: (id) => scratches.getById(id)?.title,
    repo: (id) => repos.getById(id)?.name,
    get sprint() {
      return clickup.configured
    },
  }

  // A workspace torn down, a scratch ended, a repo removed or ClickUp disconnected takes
  // its page with it.
  $effect(() => {
    if (!routeIsLive(nav.route, routeLookups))
      untrack(() => navigate(NEW_THREAD))
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

  $effect(() => {
    document.documentElement.classList.toggle(
      'reduce-motion',
      settings.values.reduceMotion,
    )
  })

  $effect(() => {
    const title = windowTitle(nav.route, routeLookups, appName)
    void getCurrentWindow()
      .setTitle(title)
      .catch(() => {
        document.title = title
      })
  })
</script>

<ModeWatcher />
<Backdrop />
<PlaceSwitcher />

<Tooltip.Provider>
  <ToastsRegion />
  {#each shellDialogIds as id (id)}
    <LazyView load={SHELL_DIALOGS[id]} when={shellDialogs.isOpen(id)} />
  {/each}
  <CommandPalette />
  <AppShell>
    {#key nav.view}
      {@const view = ROUTE_VIEWS[nav.view]}
      {#if 'load' in view}
        <LazyView load={view.load} />
      {:else}
        <view.component />
      {/if}
    {/key}
  </AppShell>
</Tooltip.Provider>
