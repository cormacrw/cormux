<script lang="ts">
  import { Command as CommandPrimitive, useId } from 'bits-ui'
  import { cn } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    children,
    heading,
    value,
    ...restProps
  }: CommandPrimitive.GroupProps & {
    heading?: string
  } = $props()
</script>

<CommandPrimitive.Group
  bind:ref
  data-slot="command-group"
  class={cn(
    'text-foreground overflow-hidden p-1',
    className,
  )}
  value={value ?? heading ?? `----${useId()}`}
  {...restProps}
>
  {#if heading}
    <CommandPrimitive.GroupHeading
      class="eyebrow px-3 pt-2 pb-1 text-muted-foreground"
    >
      {heading}
    </CommandPrimitive.GroupHeading>
  {/if}
  <CommandPrimitive.GroupItems {children} />
</CommandPrimitive.Group>
