<script lang="ts">
  import type { Component } from 'svelte'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { loadView } from './lazy-views'

  let {
    load,
    when = true,
  }: {
    load: () => Promise<{ default: Component }>
    /** Hold off until this is first true; once loaded, the component stays mounted. */
    when?: boolean
  } = $props()

  let View = $state.raw<Component>()

  function report(error: unknown) {
    console.error('Could not load a view', error)
    toastCoreError('Could not load this view. Reload the window to try again.')
  }

  $effect(() => {
    if (when && !View)
      void loadView(load, report).then((loaded) => {
        if (loaded) View = loaded
      })
  })
</script>

{#if View}
  <View />
{/if}
