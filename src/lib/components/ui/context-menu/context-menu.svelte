<script lang="ts">
  import { ContextMenu as ContextMenuPrimitive } from 'bits-ui'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'

  let {
    open = $bindable(false),
    ...restProps
  }: ContextMenuPrimitive.RootProps = $props()

  $effect(() => keyboardLayers.hold('overlay', () => open))
</script>

<ContextMenuPrimitive.Root bind:open {...restProps} />
