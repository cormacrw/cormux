<script lang="ts">
  import { Command as CommandPrimitive } from 'bits-ui'
  import type { Snippet } from 'svelte'
  import * as InputGroup from '$lib/components/ui/input-group/index.js'
  import SearchIcon from '@lucide/svelte/icons/search'
  import { cn } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    class: className,
    value = $bindable(''),
    leading,
    ...restProps
  }: CommandPrimitive.InputProps & {
    /** Rendered between the search icon and the text, e.g. a mode chip. */
    leading?: Snippet
  } = $props()
</script>

<div data-slot="command-input-wrapper" class="px-1 pt-1">
  <InputGroup.Root
    class="sunken !h-11 !rounded-[18px_16px_18px_14px] !border-0 !bg-sunken focus-within:!ring-0 focus-within:!shadow-[inset_0_4px_8px_var(--sunken-shade),0_0_0_4px_#f6c9d2] *:data-[slot=input-group-addon]:pl-3!"
  >
    <CommandPrimitive.Input
      {value}
      data-slot="command-input"
      class={cn(
        'w-full !rounded-none !bg-transparent !shadow-none text-[15px] font-semibold outline-hidden disabled:cursor-not-allowed disabled:opacity-50',
        className,
      )}
      {...restProps}
    >
      {#snippet child({ props })}
        <InputGroup.Input {...props} bind:value bind:ref />
      {/snippet}
    </CommandPrimitive.Input>
    <InputGroup.Addon>
      <SearchIcon class="size-4 shrink-0 opacity-50" />
    </InputGroup.Addon>
    {#if leading}
      <InputGroup.Addon class="pl-0">{@render leading()}</InputGroup.Addon>
    {/if}
  </InputGroup.Root>
</div>
