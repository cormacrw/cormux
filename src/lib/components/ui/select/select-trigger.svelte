<script lang="ts">
  import { Select as SelectPrimitive } from 'bits-ui'
  import ChevronDownIcon from '@lucide/svelte/icons/chevron-down'
  import { cn, type WithoutChild } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    children,
    size = 'default',
    ...restProps
  }: WithoutChild<SelectPrimitive.TriggerProps> & {
    size?: 'sm' | 'default'
  } = $props()
</script>

<SelectPrimitive.Trigger
  bind:ref
  data-slot="select-trigger"
  data-size={size}
  class={cn(
    "bg-[#a9cdeb] text-cocoa clay squish data-placeholder:text-cocoa/70 focus-visible:outline-none focus-visible:shadow-[var(--clay-shade),var(--clay-drop),0_0_0_4px_#f6c9d2] aria-invalid:shadow-[var(--clay-shade),var(--clay-drop),0_0_0_3px_var(--clay-brick)] data-[state=open]:shadow-[var(--clay-shade),var(--clay-drop),0_0_0_4px_#f6c9d2] gap-2 rounded-[22px_18px_20px_16px] border-0 py-2 pr-3 pl-4 text-[15px] font-semibold select-none data-[size=default]:h-10 data-[size=sm]:h-8 data-[size=sm]:pl-3 data-[size=sm]:text-sm *:data-[slot=select-value]:flex *:data-[slot=select-value]:gap-1.5 [&_svg:not([class*='size-'])]:size-4 flex w-fit items-center justify-between whitespace-nowrap outline-none disabled:cursor-not-allowed disabled:opacity-50 *:data-[slot=select-value]:line-clamp-1 *:data-[slot=select-value]:flex *:data-[slot=select-value]:items-center [&_svg]:pointer-events-none [&_svg]:shrink-0",
    className,
  )}
  {...restProps}
>
  {@render children?.()}
  <ChevronDownIcon class="text-cocoa size-4 pointer-events-none" strokeWidth={3} />
</SelectPrimitive.Trigger>
