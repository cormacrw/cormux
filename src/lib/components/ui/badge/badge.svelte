<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants'

  export const badgeVariants = tv({
    base: 'h-6 gap-1 rounded-full border-0 px-2.5 text-[12.5px] font-bold transition-all has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2 [&>svg]:size-3! group/badge inline-flex w-fit shrink-0 items-center justify-center overflow-hidden whitespace-nowrap focus-visible:shadow-[0_0_0_3px_var(--card),0_0_0_6px_var(--ring)] aria-invalid:ring-destructive/20 [&>svg]:pointer-events-none',
    variants: {
      variant: {
        default:
          'bg-primary text-primary-foreground shadow-[inset_0_-2px_0_rgb(0_0_0/0.12)]',
        secondary:
          'bg-pebble text-cocoa shadow-[inset_0_-2px_0_rgb(0_0_0/0.1)]',
        destructive:
          'bg-felt-brick text-brick-ink shadow-[inset_0_-2px_0_rgb(0_0_0/0.08)]',
        outline:
          'bg-transparent text-foreground outline outline-1 outline-border',
        ghost: 'hover:bg-foreground/[0.07]',
        link: 'text-foreground underline underline-offset-4 hover:no-underline',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  })

  export type BadgeVariant = VariantProps<typeof badgeVariants>['variant']
</script>

<script lang="ts">
  import { cn, type WithElementRef } from '$lib/utils.js'
  import type { HTMLAnchorAttributes } from 'svelte/elements'

  let {
    ref = $bindable(null),
    href,
    class: className,
    variant = 'default',
    children,
    ...restProps
  }: WithElementRef<HTMLAnchorAttributes> & {
    variant?: BadgeVariant
  } = $props()
</script>

<svelte:element
  this={href ? 'a' : 'span'}
  bind:this={ref}
  data-slot="badge"
  {href}
  class={cn(badgeVariants({ variant }), className)}
  {...restProps}
>
  {@render children?.()}
</svelte:element>
