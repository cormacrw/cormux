<script lang="ts">
  import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'

  let {
    open = $bindable(false),
    ...restProps
  }: DropdownMenuPrimitive.RootProps = $props()

  $effect(() => keyboardLayers.hold('overlay', () => open))
</script>

<DropdownMenuPrimitive.Root bind:open {...restProps} />
