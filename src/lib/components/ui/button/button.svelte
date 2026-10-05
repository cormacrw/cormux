<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants'
  import { cn, type WithElementRef } from '$lib/utils.js'
  import type {
    HTMLAnchorAttributes,
    HTMLButtonAttributes,
  } from 'svelte/elements'

  export const buttonVariants = tv({
    base: "aria-invalid:ring-destructive/20 aria-invalid:ring-3 font-display font-extrabold tracking-[0.005em] rounded-[24px_28px_22px_26px] border-0 [&_svg:not([class*='size-'])]:size-4 group/button inline-flex shrink-0 items-center justify-center whitespace-nowrap outline-none select-none squish focus-visible:shadow-[0_0_0_3px_var(--card),0_0_0_6px_var(--ring)] focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
    variants: {
      variant: {
        // Clay in the felt's primary colour.
        default:
          'clay bg-primary text-primary-foreground hover:brightness-[1.04] focus-visible:shadow-[var(--clay-shade),0_0_0_3px_var(--card),0_0_0_6px_var(--ring)]',
        // White clay: the everyday secondary.
        outline:
          'clay bg-secondary text-secondary-foreground aria-expanded:bg-custard aria-expanded:text-cocoa focus-visible:shadow-[var(--clay-shade),0_0_0_3px_var(--card),0_0_0_6px_var(--ring)]',
        secondary:
          'clay bg-secondary text-secondary-foreground aria-expanded:bg-custard aria-expanded:text-cocoa focus-visible:shadow-[var(--clay-shade),0_0_0_3px_var(--card),0_0_0_6px_var(--ring)]',
        // Bare text that picks up a soft felt patch on hover.
        ghost:
          'hover:bg-foreground/[0.07] aria-expanded:bg-foreground/[0.07] [&:hover:not(:disabled)]:transform-none [&:active:not(:disabled)]:scale-[0.97]',
        // Brick clay with dark brick text.
        destructive:
          'clay bg-brick text-[#8a1f14] focus-visible:shadow-[var(--clay-shade),0_0_0_3px_var(--card),0_0_0_6px_var(--ring)]',
        link: 'font-sans font-bold text-foreground underline underline-offset-4 decoration-2 decoration-stitch hover:decoration-current [&:hover:not(:disabled)]:transform-none',
      },
      size: {
        default:
          'h-9 gap-1.5 px-4 text-[15px] has-data-[icon=inline-end]:pr-3 has-data-[icon=inline-start]:pl-3',
        xs: "h-7 gap-1 px-2.5 text-xs has-data-[icon=inline-end]:pr-2 has-data-[icon=inline-start]:pl-2 [&_svg:not([class*='size-'])]:size-3",
        sm: "h-8 gap-1 px-3.5 text-sm has-data-[icon=inline-end]:pr-2.5 has-data-[icon=inline-start]:pl-2.5 [&_svg:not([class*='size-'])]:size-3.5",
        lg: 'h-11 gap-2 px-5 text-base has-data-[icon=inline-end]:pr-4 has-data-[icon=inline-start]:pl-4',
        xl: 'h-12 gap-2 px-6 text-base has-data-[icon=inline-end]:pr-5 has-data-[icon=inline-start]:pl-5',
        icon: 'size-9 rounded-full',
        'icon-xs':
          "size-7 rounded-full [&_svg:not([class*='size-'])]:size-3",
        'icon-sm': 'size-8 rounded-full',
        'icon-lg': 'size-11 rounded-full',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  })

  export type ButtonVariant = VariantProps<typeof buttonVariants>['variant']
  export type ButtonSize = VariantProps<typeof buttonVariants>['size']

  export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
    WithElementRef<HTMLAnchorAttributes> & {
      variant?: ButtonVariant
      size?: ButtonSize
    }
</script>

<script lang="ts">
  let {
    class: className,
    variant = 'default',
    size = 'default',
    ref = $bindable(null),
    href = undefined,
    type = 'button',
    disabled,
    children,
    ...restProps
  }: ButtonProps = $props()
</script>

{#if href}
  <a
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    href={disabled ? undefined : href}
    aria-disabled={disabled}
    role={disabled ? 'link' : undefined}
    tabindex={disabled ? -1 : undefined}
    {...restProps}
  >
    {@render children?.()}
  </a>
{:else}
  <button
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    {type}
    {disabled}
    {...restProps}
  >
    {@render children?.()}
  </button>
{/if}
