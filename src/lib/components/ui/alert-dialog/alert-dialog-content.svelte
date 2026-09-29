<script lang="ts">
  import { AlertDialog as AlertDialogPrimitive } from 'bits-ui'
  import {
    cn,
    type WithoutChild,
    type WithoutChildrenOrChild,
  } from '$lib/utils.js'
  import AlertDialogOverlay from './alert-dialog-overlay.svelte'
  import AlertDialogPortal from './alert-dialog-portal.svelte'
  import type { ComponentProps } from 'svelte'

  let {
    ref = $bindable(null),
    class: className,
    size = 'default',
    portalProps,
    ...restProps
  }: WithoutChild<AlertDialogPrimitive.ContentProps> & {
    size?: 'default' | 'sm'
    portalProps?: WithoutChildrenOrChild<
      ComponentProps<typeof AlertDialogPortal>
    >
  } = $props()
</script>

<AlertDialogPortal {...portalProps}>
  <AlertDialogOverlay />
  <AlertDialogPrimitive.Content
    data-slot="alert-dialog-content"
    data-size={size}
    class={cn(
      'data-open:animate-in data-closed:animate-out data-closed:fade-out-0 data-open:fade-in-0 data-closed:zoom-out-95 data-open:zoom-in-95 bg-popover text-popover-foreground ring-foreground/15 gap-5 rounded-xl p-5 ring-1 shadow-[0_32px_80px_rgba(0,0,0,.6)] duration-200 data-[size=default]:max-w-xs data-[size=sm]:max-w-xs data-[size=default]:sm:max-w-sm group/alert-dialog-content fixed top-1/2 left-1/2 z-50 grid w-full -translate-x-1/2 -translate-y-1/2 outline-none',
      className,
    )}
    {...restProps}
    bind:ref
  />
</AlertDialogPortal>
