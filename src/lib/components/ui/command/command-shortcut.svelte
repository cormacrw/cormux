<script lang="ts">
  import { cn, type WithElementRef } from '$lib/utils.js'
  import type { HTMLAttributes } from 'svelte/elements'

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLSpanElement>> = $props()
</script>

<span
  bind:this={ref}
  data-slot="command-shortcut"
  class={cn(
    'text-muted-foreground group-data-selected/command-item:text-cocoa/70 ml-auto font-mono text-[12px] font-semibold tracking-normal',
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</span>
