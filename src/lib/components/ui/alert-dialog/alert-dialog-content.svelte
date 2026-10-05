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
      'data-open:clay-pop data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95 bg-card text-card-foreground gap-5 rounded-[34px_28px_32px_26px/28px_34px_26px_32px] p-7 text-[15px] outline outline-dashed -outline-offset-6 outline-stitch shadow-lift-5 duration-200 data-[size=default]:max-w-xs data-[size=sm]:max-w-xs data-[size=default]:sm:max-w-sm group/alert-dialog-content fixed top-1/2 left-1/2 z-50 grid w-full -translate-x-1/2 -translate-y-1/2 outline-none',
      className,
    )}
    {...restProps}
    bind:ref
  />
</AlertDialogPortal>
