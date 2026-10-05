<script lang="ts">
  import { Dialog as DialogPrimitive } from 'bits-ui'
  import { Button } from '$lib/components/ui/button/index.js'
  import { cn, type WithElementRef } from '$lib/utils.js'
  import type { HTMLAttributes } from 'svelte/elements'

  let {
    ref = $bindable(null),
    class: className,
    children,
    showCloseButton = false,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
    showCloseButton?: boolean
  } = $props()
</script>

<div
  bind:this={ref}
  data-slot="dialog-footer"
  class={cn(
    '-mx-7 -mb-7 border-t border-border px-7 pt-5 pb-7 flex flex-col-reverse gap-3 sm:flex-row sm:items-center sm:justify-end',
    className,
  )}
  {...restProps}
>
  {@render children?.()}
  {#if showCloseButton}
    <DialogPrimitive.Close>
      {#snippet child({ props })}
        <Button variant="outline" {...props}>Close</Button>
      {/snippet}
    </DialogPrimitive.Close>
  {/if}
</div>
