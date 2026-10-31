<script lang="ts">
  import type { Component } from 'svelte'

  let {
    load,
    when = true,
  }: {
    load: () => Promise<{ default: Component }>
    /** Hold off until this is first true; once loaded, the component stays mounted. */
    when?: boolean
  } = $props()

  let View = $state.raw<Component>()

  $effect(() => {
    if (when) void load().then((module) => (View = module.default))
  })
</script>

{#if View}
  <View />
{/if}
