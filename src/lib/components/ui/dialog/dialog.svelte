<script lang="ts">
  import { Dialog as DialogPrimitive } from 'bits-ui'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'

  let {
    open = $bindable(false),
    layer = 'modal',
    ...restProps
  }: DialogPrimitive.RootProps & {
    /** How it holds the keyboard while open: a modal makes shortcuts wait, an overlay doesn't. */
    layer?: 'modal' | 'overlay'
  } = $props()

  $effect(() => keyboardLayers.hold(layer, () => open))
</script>

<DialogPrimitive.Root bind:open {...restProps} />
