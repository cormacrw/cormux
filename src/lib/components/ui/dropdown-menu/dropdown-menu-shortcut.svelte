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
  data-slot="dropdown-menu-shortcut"
  class={cn(
    'text-muted-foreground group-focus/dropdown-menu-item:text-accent-foreground ml-auto pl-4 font-mono text-[11.5px] font-medium tracking-normal',
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</span>
