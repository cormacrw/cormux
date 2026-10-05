<script lang="ts">
  import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui'
  import { cn } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    inset,
    variant = 'default',
    ...restProps
  }: DropdownMenuPrimitive.ItemProps & {
    inset?: boolean
    variant?: 'default' | 'destructive'
  } = $props()
</script>

<DropdownMenuPrimitive.Item
  bind:ref
  data-slot="dropdown-menu-item"
  data-inset={inset}
  data-variant={variant}
  class={cn(
    "focus:bg-accent focus:text-accent-foreground focus:shadow-[inset_0_-3px_0_rgb(0_0_0/0.12),inset_0_2px_3px_rgb(255_255_255/0.55)] data-[variant=destructive]:text-destructive data-[variant=destructive]:focus:bg-brick dark:data-[variant=destructive]:focus:text-[#8a1f14] data-[variant=destructive]:focus:text-destructive data-[variant=destructive]:*:[svg]:text-destructive not-data-[variant=destructive]:focus:**:text-accent-foreground gap-2.5 rounded-[14px_12px_14px_11px] px-3 py-2 text-[15px] font-semibold data-inset:pl-7 [&_svg:not([class*='size-'])]:size-4 group/dropdown-menu-item relative flex cursor-default items-center outline-hidden select-none data-disabled:pointer-events-none data-disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
    className,
  )}
  {...restProps}
/>
