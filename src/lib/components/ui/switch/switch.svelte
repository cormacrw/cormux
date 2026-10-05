<script lang="ts">
  import { Switch as SwitchPrimitive } from 'bits-ui'
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    checked = $bindable(false),
    size = 'default',
    ...restProps
  }: WithoutChildrenOrChild<SwitchPrimitive.RootProps> & {
    size?: 'sm' | 'default'
  } = $props()
</script>

<SwitchPrimitive.Root
  bind:ref
  bind:checked
  data-slot="switch"
  data-size={size}
  class={cn(
    'data-checked:bg-leaf data-unchecked:bg-pebble shadow-[inset_0_-3px_0_rgb(0_0_0/0.14),inset_0_2px_4px_rgb(0_0_0/0.12)] focus-visible:outline-none focus-visible:shadow-[0_0_0_3px_var(--card),0_0_0_6px_var(--ring)] aria-invalid:ring-destructive/20 aria-invalid:ring-3 shrink-0 rounded-full border-0 px-[3px] data-[size=default]:h-7 data-[size=default]:w-12 data-[size=sm]:h-5 data-[size=sm]:w-9 peer group/switch relative inline-flex items-center transition-all outline-none after:absolute after:-inset-x-3 after:-inset-y-2 data-disabled:cursor-not-allowed data-disabled:opacity-50',
    className,
  )}
  {...restProps}
>
  <SwitchPrimitive.Thumb
    data-slot="switch-thumb"
    class={cn(
      'bg-[#fffaf1] shadow-[inset_0_-2px_0_rgb(0_0_0/0.12),0_2px_4px_rgb(30_50_60/0.25)] rounded-full group-data-[size=default]/switch:size-[22px] group-data-[size=sm]/switch:size-3.5 group-data-[size=default]/switch:data-checked:translate-x-[20px] group-data-[size=sm]/switch:data-checked:translate-x-[16px] transition-transform duration-200 ease-[var(--squish)] group-data-[size=default]/switch:data-unchecked:translate-x-0 group-data-[size=sm]/switch:data-unchecked:translate-x-0 pointer-events-none block ring-0 transition-transform',
      'rtl:data-[state=checked]:translate-x-[calc(-100%)]',
    )}
  />
</SwitchPrimitive.Root>
