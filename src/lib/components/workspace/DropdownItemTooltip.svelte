<script lang="ts">
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import type { Snippet } from 'svelte'

  let {
    label,
    tooltip,
    disabled = false,
    variant = 'default' as 'default' | 'destructive',
    onclick,
    odId,
    children,
  }: {
    label: string
    tooltip?: string
    disabled?: boolean
    variant?: 'default' | 'destructive'
    onclick?: () => void
    odId?: string
    children?: Snippet
  } = $props()
</script>

{#if disabled && tooltip}
  <Tooltip.Root>
    <Tooltip.Trigger class="flex w-full">
      {#snippet child({ props })}
        <span {...props} class="flex w-full">
          <DropdownMenu.Item
            {disabled}
            {variant}
            class="w-full"
            {onclick}
            data-od-id={odId}
          >
            {@render children?.()}
            <span class="grow truncate">{label}</span>
          </DropdownMenu.Item>
        </span>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content side="left">{tooltip}</Tooltip.Content>
  </Tooltip.Root>
{:else}
  <DropdownMenu.Item
    {disabled}
    {variant}
    title={tooltip}
    {onclick}
    class="w-full"
    data-od-id={odId}
  >
    {@render children?.()}
    <span class="grow truncate">{label}</span>
  </DropdownMenu.Item>
{/if}
