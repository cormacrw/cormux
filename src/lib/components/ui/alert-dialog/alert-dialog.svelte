<script lang="ts">
  import { AlertDialog as AlertDialogPrimitive } from 'bits-ui'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'

  let {
    open = $bindable(false),
    ...restProps
  }: AlertDialogPrimitive.RootProps = $props()

  $effect(() => keyboardLayers.hold('modal', () => open))
</script>

<AlertDialogPrimitive.Root bind:open {...restProps} />
