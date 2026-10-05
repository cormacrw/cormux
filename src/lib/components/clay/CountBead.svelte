<script lang="ts">
  import { cn } from '$lib/utils'

  /** A marigold clay count, for approvals that need you. It wiggles; nothing else does. */
  let {
    count,
    wiggle = true,
    class: className = '',
  }: { count: number; wiggle?: boolean; class?: string } = $props()
</script>

<span
  class={cn(
    'inline-grid size-6 shrink-0 place-items-center rounded-full bg-marigold font-display text-[13px] leading-none font-extrabold text-cocoa shadow-[inset_0_-2px_0_rgb(0_0_0/0.2),0_2px_4px_rgb(30_50_60/0.25)]',
    wiggle && 'clay-wiggle',
    className,
  )}
  aria-hidden="true">{count}</span
>
