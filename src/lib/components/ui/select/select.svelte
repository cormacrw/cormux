<script lang="ts">
  import { Select as SelectPrimitive } from 'bits-ui'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'

  let {
    open = $bindable(false),
    value = $bindable(),
    ...restProps
  }: SelectPrimitive.RootProps = $props()

  $effect(() => keyboardLayers.hold('overlay', () => open))
</script>

<SelectPrimitive.Root bind:open bind:value={value as never} {...restProps} />
