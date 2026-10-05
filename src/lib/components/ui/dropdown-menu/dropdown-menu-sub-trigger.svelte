<script lang="ts">
  import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui'
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right'
  import { cn } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    inset,
    children,
    ...restProps
  }: DropdownMenuPrimitive.SubTriggerProps & {
    inset?: boolean
  } = $props()
</script>

<DropdownMenuPrimitive.SubTrigger
  bind:ref
  data-slot="dropdown-menu-sub-trigger"
  data-inset={inset}
  class={cn(
    "focus:bg-accent focus:text-accent-foreground focus:shadow-[inset_0_-3px_0_rgb(0_0_0/0.12),inset_0_2px_3px_rgb(255_255_255/0.55)] data-open:bg-accent data-open:text-accent-foreground not-data-[variant=destructive]:focus:**:text-accent-foreground gap-2.5 rounded-[14px_12px_14px_11px] px-3 py-2 text-[15px] font-semibold data-inset:pl-7 [&_svg:not([class*='size-'])]:size-4 flex cursor-default items-center outline-hidden select-none [&_svg]:pointer-events-none [&_svg]:shrink-0",
    className,
  )}
  {...restProps}
>
  {@render children?.()}
  <ChevronRightIcon class="cn-rtl-flip ml-auto" />
</DropdownMenuPrimitive.SubTrigger>
