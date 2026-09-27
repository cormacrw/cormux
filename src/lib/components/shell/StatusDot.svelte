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
      'relative size-2 rounded-full',
      variant === 'running' &&
        'bg-success after:absolute after:-inset-1 after:rounded-full after:border after:border-success/60 motion-safe:after:animate-[status-ring_2s_ease-out_infinite]',
      variant === 'idle' && 'bg-muted-foreground/50',
      variant === 'provisioning' &&
        'bg-info motion-safe:animate-[status-blink_1.2s_ease-in-out_infinite]',
      variant === 'paused' && 'bg-warning',
    )}
  ></span>
</span>

<style>
  @keyframes status-ring {
    from {
      opacity: 0.9;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(1.6);
    }
  }

  @keyframes status-blink {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }
</style>
