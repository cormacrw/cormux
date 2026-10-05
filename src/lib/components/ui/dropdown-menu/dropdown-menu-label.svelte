<script lang="ts">
  import { cn, type WithElementRef } from '$lib/utils.js'
  import type { HTMLAttributes } from 'svelte/elements'

  let {
    ref = $bindable(null),
    class: className,
    inset,
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
    inset?: boolean
  } = $props()
</script>

<div
  bind:this={ref}
  data-slot="dropdown-menu-label"
  data-inset={inset}
  class={cn(
    'eyebrow text-muted-foreground px-3 pt-2 pb-1 text-[11.5px] data-inset:pl-7',
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</div>
