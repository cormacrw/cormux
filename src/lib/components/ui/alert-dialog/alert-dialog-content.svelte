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
      'data-open:animate-in data-closed:animate-out data-closed:fade-out-0 data-open:fade-in-0 data-closed:zoom-out-[0.97] data-open:zoom-in-[0.97] bg-popover text-popover-foreground ring-white/12 dark:ring-white/12 gap-5 rounded-2xl p-5 ring-1 shadow-[inset_0_1px_0_var(--glass-highlight),0_24px_64px_rgba(0,0,0,.18)] dark:shadow-[inset_0_1px_0_var(--glass-highlight),0_32px_80px_rgba(0,0,0,.55)] duration-150 data-[size=default]:max-w-xs data-[size=sm]:max-w-xs data-[size=default]:sm:max-w-sm group/alert-dialog-content fixed top-1/2 left-1/2 z-50 grid w-full -translate-x-1/2 -translate-y-1/2 outline-none',
      className,
    )}
    {...restProps}
    bind:ref
  />
</AlertDialogPortal>
