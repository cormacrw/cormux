<script lang="ts">
  import { Command as CommandPrimitive } from 'bits-ui'
  import CheckIcon from '@lucide/svelte/icons/check'
  import { cn } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: CommandPrimitive.ItemProps = $props()
</script>

<CommandPrimitive.Item
  bind:ref
  data-slot="command-item"
  class={cn(
    "data-selected:bg-custard data-selected:text-cocoa data-selected:*:[svg]:text-cocoa relative flex cursor-default items-center gap-2.5 rounded-[16px_14px_16px_12px] px-3 py-2.5 text-[15px] font-semibold outline-hidden select-none [&_svg:not([class*='size-'])]:size-4 group/command-item data-[disabled=true]:pointer-events-none data-[disabled=true]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
    className,
  )}
  {...restProps}
>
  {@render children?.()}
  <CheckIcon
    class="absolute right-3 size-4 opacity-0 group-has-data-[slot=command-shortcut]/command-item:hidden group-data-[checked=true]/command-item:opacity-100"
  />
</CommandPrimitive.Item>
