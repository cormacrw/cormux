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

<div data-slot="command-input-wrapper" class="p-1 pb-0">
  <InputGroup.Root
    class="bg-input/30 border-input/30 h-8! rounded-lg! shadow-none! *:data-[slot=input-group-addon]:pl-2!"
  >
    <CommandPrimitive.Input
      {value}
      data-slot="command-input"
      class={cn(
        'w-full text-sm outline-hidden disabled:cursor-not-allowed disabled:opacity-50',
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
