<script lang="ts">
  import { Checkbox as CheckboxPrimitive } from 'bits-ui'
  import CheckIcon from '@lucide/svelte/icons/check'
  import MinusIcon from '@lucide/svelte/icons/minus'
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js'

  let {
    ref = $bindable(null),
    checked = $bindable(false),
    indeterminate = $bindable(false),
    class: className,
    ...restProps
  }: WithoutChildrenOrChild<CheckboxPrimitive.RootProps> = $props()
</script>

<CheckboxPrimitive.Root
  bind:ref
  data-slot="checkbox"
  class={cn(
    'bg-sunken shadow-[inset_0_2px_4px_var(--sunken-shade),0_0_0_1.5px_var(--input)] data-checked:bg-primary data-checked:text-primary-foreground data-checked:shadow-[inset_0_-2px_0_rgb(0_0_0/0.15),inset_0_2px_3px_rgb(255_255_255/0.55),0_2px_4px_rgb(30_50_60/0.2)] aria-invalid:ring-destructive/20 flex size-[22px] items-center justify-center rounded-[7px_6px_7px_5px] border-0 transition-[background-color,box-shadow] group-has-disabled/field:opacity-50 focus-visible:outline-none focus-visible:shadow-[0_0_0_3px_var(--card),0_0_0_6px_var(--ring)] aria-invalid:ring-3 peer relative shrink-0 outline-none after:absolute after:-inset-x-3 after:-inset-y-2 disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
  bind:checked
  bind:indeterminate
  {...restProps}
>
  {#snippet children({ checked, indeterminate })}
    <div
      data-slot="checkbox-indicator"
      class="[&>svg]:size-4 [&>svg]:stroke-[3.5] grid place-content-center text-current transition-none"
    >
      {#if checked}
        <CheckIcon />
      {:else if indeterminate}
        <MinusIcon />
      {/if}
    </div>
  {/snippet}
</CheckboxPrimitive.Root>
