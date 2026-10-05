<script lang="ts">
  import { Dialog as DialogPrimitive } from 'bits-ui'
  import XIcon from '@lucide/svelte/icons/x'
  import { Button } from '$lib/components/ui/button/index.js'
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js'
  import * as Dialog from './index.js'
  import DialogPortal from './dialog-portal.svelte'
  import type { Snippet } from 'svelte'
  import type { ComponentProps } from 'svelte'

  let {
    ref = $bindable(null),
    class: className,
    portalProps,
    children,
    showCloseButton = true,
    ...restProps
  }: WithoutChildrenOrChild<DialogPrimitive.ContentProps> & {
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof DialogPortal>>
    children: Snippet
    showCloseButton?: boolean
  } = $props()
</script>

<DialogPortal {...portalProps}>
  <Dialog.Overlay />
  <DialogPrimitive.Content
    data-slot="dialog-content"
    class={cn(
      'bg-card text-card-foreground data-open:clay-pop data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95 grid max-w-[calc(100%_-_2rem)] gap-5 rounded-[34px_28px_32px_26px/28px_34px_26px_32px] p-7 text-[15px] outline outline-dashed -outline-offset-6 outline-stitch shadow-lift-5 duration-200 sm:max-w-sm fixed top-1/2 left-1/2 z-50 w-full -translate-x-1/2 -translate-y-1/2 outline-none',
      className,
    )}
    {...restProps}
    bind:ref
  >
    {@render children?.()}
    {#if showCloseButton}
      <DialogPrimitive.Close data-slot="dialog-close">
        {#snippet child({ props })}
          <Button
            variant="secondary"
            class="absolute top-6 right-6"
            size="icon-sm"
            {...props}
          >
            <XIcon />
            <span class="sr-only">Close</span>
          </Button>
        {/snippet}
      </DialogPrimitive.Close>
    {/if}
  </DialogPrimitive.Content>
</DialogPortal>
