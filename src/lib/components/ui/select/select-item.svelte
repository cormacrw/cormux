<script lang="ts">
  import { Select as SelectPrimitive } from 'bits-ui'
  import CheckIcon from '@lucide/svelte/icons/check'
  import { cn, type WithoutChild } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    value,
    label,
    children: childrenProp,
    ...restProps
  }: WithoutChild<SelectPrimitive.ItemProps> = $props()
</script>

<SelectPrimitive.Item
  bind:ref
  {value}
  {label}
  data-slot="select-item"
  class={cn(
    "data-highlighted:bg-custard data-highlighted:text-cocoa gap-2 rounded-[14px_12px_14px_11px] py-2 pr-9 pl-3 text-[15px] font-semibold [&_svg:not([class*='size-'])]:size-4 *:[span]:last:flex *:[span]:last:items-center *:[span]:last:gap-2 relative flex w-full cursor-default items-center outline-hidden select-none data-disabled:pointer-events-none data-disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
    className,
  )}
  {...restProps}
>
  {#snippet children({ selected, highlighted })}
    <span
      class="pointer-events-none absolute right-3 flex size-4 items-center justify-center"
    >
      {#if selected}
        <CheckIcon class="pointer-events-none" />
      {/if}
    </span>
    <span class="flex flex-1 gap-2 shrink-0 whitespace-nowrap">
      {#if childrenProp}
        {@render childrenProp({ selected, highlighted })}
      {:else}
        {label || value}
      {/if}
    </span>
  {/snippet}
</SelectPrimitive.Item>
