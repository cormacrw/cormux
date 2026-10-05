<script lang="ts">
  import { cn, type WithElementRef } from '$lib/utils.js'
  import type { HTMLAttributes } from 'svelte/elements'

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLElement>> = $props()
</script>

<kbd
  bind:this={ref}
  data-slot="kbd"
  class={cn(
    "bg-secondary text-foreground shadow-[inset_0_-2px_0_rgb(0_0_0/0.1),0_1px_2px_rgb(30_50_60/0.15)] in-data-[slot=tooltip-content]:bg-white/15 in-data-[slot=tooltip-content]:text-current in-data-[slot=tooltip-content]:shadow-none in-data-[slot=button]:bg-transparent in-data-[slot=button]:shadow-none in-data-[slot=button]:text-current/70 in-data-[slot=button]:px-0.5 h-6 w-fit min-w-6 gap-1 rounded-full px-1.5 font-mono text-[11.5px] font-semibold [&_svg:not([class*='size-'])]:size-3 pointer-events-none inline-flex items-center justify-center select-none",
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</kbd>
