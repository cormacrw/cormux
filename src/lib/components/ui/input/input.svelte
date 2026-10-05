<script lang="ts">
  import { cn, type WithElementRef } from '$lib/utils.js'
  import type {
    HTMLInputAttributes,
    HTMLInputTypeAttribute,
  } from 'svelte/elements'

  type InputType = Exclude<HTMLInputTypeAttribute, 'file'>

  type Props = WithElementRef<
    Omit<HTMLInputAttributes, 'type'> &
      (
        | { type: 'file'; files?: FileList }
        | { type?: InputType; files?: undefined }
      )
  >

  let {
    ref = $bindable(null),
    value = $bindable(),
    type,
    files = $bindable(),
    class: className,
    'data-slot': dataSlot = 'input',
    ...restProps
  }: Props = $props()
</script>

{#if type === 'file'}
  <input
    bind:this={ref}
    data-slot={dataSlot}
    class={cn(
      'sunken disabled:opacity-60 h-10 rounded-[22px_18px_20px_16px] border-0 px-3.5 py-1 text-base transition-shadow file:h-6 file:text-sm file:font-medium md:text-[15px] w-full min-w-0 file:inline-flex file:border-0 file:bg-transparent file:text-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed',
      className,
    )}
    type="file"
    bind:files
    bind:value
    {...restProps}
  />
{:else}
  <input
    bind:this={ref}
    data-slot={dataSlot}
    class={cn(
      'sunken disabled:opacity-60 h-10 rounded-[22px_18px_20px_16px] border-0 px-3.5 py-1 text-base transition-shadow file:h-6 file:text-sm file:font-medium md:text-[15px] w-full min-w-0 file:inline-flex file:border-0 file:bg-transparent file:text-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed',
      className,
    )}
    {type}
    bind:value
    {...restProps}
  />
{/if}
