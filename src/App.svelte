<script lang="ts">
  import { onMount } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import AppShell from '$lib/components/shell/AppShell.svelte'
  import { bindNativeMenu } from '$lib/components/shell/menu'
  import { fetchSnapshot, listenForStateChanges } from '$lib/ipc'
  import {
    app,
    hydrateFromSnapshot,
    patchFromEvent,
    settings,
  } from '$lib/state'
  import Homebase from './routes/Homebase.svelte'
  import Settings from './routes/Settings.svelte'
  import Workspace from './routes/Workspace.svelte'

  function onGlobalKeydown(event: KeyboardEvent) {
    if (!event.metaKey) return
    const key = event.key.toLowerCase()
    if (key === 'k') {
      event.preventDefault()
      app.requestCommandPalette()
    }
    if (key === 'n') {
      event.preventDefault()
      app.requestNewWorkspace()
    }
  }

  onMount(() => {
    let unlisten: (() => void) | undefined

    void (async () => {
      try {
        hydrateFromSnapshot(await fetchSnapshot())
      } catch (error) {
        console.warn('snapshot unavailable, using empty stores', error)
      }

      const unlistenMenu = await bindNativeMenu()
      const unlistenState = await listenForStateChanges({
        lastVersion: () => app.version,
        onEvent: (event) => {
          void patchFromEvent(event)
        },
        onSnapshot: hydrateFromSnapshot,
      })

      unlisten = () => {
        unlistenMenu()
        unlistenState()
      }
    })()

    return () => {
      unlisten?.()
    }
  })

  onMount(() => {
    window.addEventListener('keydown', onGlobalKeydown)
    return () => window.removeEventListener('keydown', onGlobalKeydown)
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

<AppShell>
  {#if app.view === 'settings'}
    <Settings />
  {:else if app.view === 'workspace'}
    <Workspace />
  {:else}
    <Homebase />
  {/if}
</AppShell>
