<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants'

  export const toggleVariants = tv({
    base: "text-foreground/75 hover:text-foreground data-[state=on]:text-foreground data-[state=on]:bg-secondary data-[state=on]:shadow-[inset_0_-3px_0_rgb(0_0_0/0.1),0_3px_6px_rgb(30_50_60/0.18)] aria-pressed:bg-secondary aria-pressed:text-foreground focus-visible:outline-none focus-visible:shadow-[0_0_0_3px_var(--card),0_0_0_6px_var(--ring)] aria-invalid:ring-destructive/20 gap-1.5 rounded-full text-[15px] font-bold transition-[background-color,box-shadow,color,transform] duration-150 [&_svg:not([class*='size-'])]:size-4 group/toggle inline-flex items-center justify-center whitespace-nowrap outline-none disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 active:scale-[0.96]",
    variants: {
      variant: {
        default: 'bg-transparent',
        outline: 'bg-transparent',
      },
      size: {
        default:
          'h-9 min-w-9 px-4 has-data-[icon=inline-end]:pr-3 has-data-[icon=inline-start]:pl-3',
        sm: "h-8 min-w-8 px-3.5 text-sm has-data-[icon=inline-end]:pr-2.5 has-data-[icon=inline-start]:pl-2.5 [&_svg:not([class*='size-'])]:size-3.5",
        lg: 'h-10 min-w-10 px-4 has-data-[icon=inline-end]:pr-3 has-data-[icon=inline-start]:pl-3',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  })

  export type ToggleVariant = VariantProps<typeof toggleVariants>['variant']
  export type ToggleSize = VariantProps<typeof toggleVariants>['size']
  export type ToggleVariants = VariantProps<typeof toggleVariants>
</script>

<script lang="ts">
  import { Toggle as TogglePrimitive } from 'bits-ui'
  import { cn } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    pressed = $bindable(false),
    class: className,
    size = 'default',
    variant = 'default',
    ...restProps
  }: TogglePrimitive.RootProps & {
    variant?: ToggleVariant
    size?: ToggleSize
  } = $props()
</script>

<TogglePrimitive.Root
  bind:ref
  bind:pressed
  data-slot="toggle"
  class={cn(toggleVariants({ variant, size }), className)}
  {...restProps}
/>
