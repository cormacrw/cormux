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
      'bg-popover text-popover-foreground data-open:animate-in data-closed:animate-out data-closed:fade-out-0 data-open:fade-in-0 data-closed:zoom-out-[0.97] data-open:zoom-in-[0.97] ring-white/12 dark:ring-white/12 grid max-w-[calc(100%_-_2rem)] gap-5 rounded-2xl p-5 text-sm ring-1 shadow-[inset_0_1px_0_var(--glass-highlight),0_24px_64px_rgba(0,0,0,.18)] dark:shadow-[inset_0_1px_0_var(--glass-highlight),0_32px_80px_rgba(0,0,0,.55)] duration-150 sm:max-w-sm fixed top-1/2 left-1/2 z-50 w-full -translate-x-1/2 -translate-y-1/2 outline-none',
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
            variant="ghost"
            class="absolute top-4 right-4"
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
