<script lang="ts">
  import { cn } from '$lib/utils'

  let { class: className }: { class?: string } = $props()
</script>

<!-- Heartbeat dot in the accent color, in step with .agent-working-text; the global reduce-motion rules stop it. -->
<span class={cn('agent-spinner', className)} aria-hidden="true"></span>

<style>
  .agent-spinner {
    /* Graphite (no data-accent) is grey, so fall back to the app's blue accent. */
    --color-1: var(--sidebar-primary);
    /* A lighter shade of the same hue, so the beat reads as a glow, not a flash. */
    --color-2: color-mix(in oklch, var(--color-1) 70%, white);
    --size: 0.5px;

    display: inline-block;
    flex-shrink: 0;
    width: calc(24 * var(--size));
    height: calc(24 * var(--size));
    border-radius: 50%;
    background: var(--color-1);
    box-shadow: 0 0 0 0 var(--color-1);
    animation: agent-heartbeat var(--agent-beat-duration) ease-in-out infinite;
  }

  :global(:root[data-accent]) .agent-spinner {
    --color-1: var(--primary);
  }

  @keyframes agent-heartbeat {
    0%,
    60%,
    100% {
      transform: scale(1);
      box-shadow: 0 0 0 0 color-mix(in oklch, var(--color-1) 40%, transparent);
      background: var(--color-1);
    }
    10% {
      transform: scale(1.35);
      background: var(--color-2);
    }
    20% {
      transform: scale(1);
    }
    30% {
      transform: scale(1.35);
      background: var(--color-2);
    }
    50% {
      transform: scale(1);
      box-shadow: 0 0 0 calc(20 * var(--size)) transparent;
    }
  }
</style>
