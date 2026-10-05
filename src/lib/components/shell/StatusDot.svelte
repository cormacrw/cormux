<script lang="ts">
  import type { StatusDotVariant } from '$lib/sidebar/status'
  import { cn } from '$lib/utils'

  let {
    variant,
    class: className = '',
  }: { variant: StatusDotVariant; class?: string } = $props()
</script>

<span
  class={cn('inline-flex size-4 items-center justify-center', className)}
  aria-hidden="true"
>
  <span
    class={cn(
      'bead size-3',
      variant === 'running' && 'clay-glow bg-leaf',
      variant === 'idle' && 'bg-pebble',
      variant === 'provisioning' &&
        'bg-pond motion-safe:animate-[status-blink_1.2s_ease-in-out_infinite]',
      variant === 'paused' && 'bg-coral',
    )}
  ></span>
</span>

<style>
  @keyframes status-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.45;
    }
  }
</style>
