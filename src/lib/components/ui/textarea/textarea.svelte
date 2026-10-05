<script lang="ts">
  import { cn, type WithElementRef, type WithoutChildren } from '$lib/utils.js'
  import type { HTMLTextareaAttributes } from 'svelte/elements'

  let {
    ref = $bindable(null),
    value = $bindable(),
    class: className,
    'data-slot': dataSlot = 'textarea',
    ...restProps
  }: WithoutChildren<WithElementRef<HTMLTextareaAttributes>> = $props()
</script>

<textarea
  bind:this={ref}
  data-slot={dataSlot}
  class={cn(
    'sunken rounded-[22px_18px_20px_16px] border-0 px-4 py-3 text-base transition-shadow md:text-[15px] flex field-sizing-content min-h-16 w-full placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-60',
    className,
  )}
  bind:value
  {...restProps}></textarea>
