<script lang="ts">
  import { ContextMenu as ContextMenuPrimitive } from 'bits-ui'
  import { cn } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    ...restProps
  }: ContextMenuPrimitive.ItemProps = $props()
</script>

<ContextMenuPrimitive.Item
  bind:ref
  data-slot="context-menu-item"
  class={cn(
    "focus:bg-accent focus:text-accent-foreground focus:**:text-accent-foreground gap-2 rounded-md px-2.5 py-1.5 text-sm [&_svg:not([class*='size-'])]:size-4 relative flex cursor-default items-center outline-hidden select-none data-disabled:pointer-events-none data-disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
    className,
  )}
  {...restProps}
/>
