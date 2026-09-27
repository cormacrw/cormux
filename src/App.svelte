<script lang="ts">
  import { onMount } from 'svelte'
  import AppShell from '$lib/components/shell/AppShell.svelte'
  import { fetchSnapshot, listenForStateChanges } from '$lib/ipc'
  import { app, hydrateFromSnapshot, patchFromEvent, settings } from '$lib/state'
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

      unlisten = await listenForStateChanges({
        lastVersion: () => app.version,
        onEvent: patchFromEvent,
        onSnapshot: hydrateFromSnapshot,
      })
    })()

    return () => {
      unlisten?.()
    }
  })

  $effect(() => {
    document.documentElement.classList.toggle('reduce-motion', settings.reduceMotion)
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
