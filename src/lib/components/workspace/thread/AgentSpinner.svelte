<script lang="ts">
  import { cn } from '$lib/utils'

  let { class: className }: { class?: string } = $props()
</script>

<!-- Heartbeat dot in the accent color, in step with .agent-working-text; the global reduce-motion rules stop it. -->
<span class={cn('agent-spinner', className)} aria-hidden="true"></span>

<style>
  /* Only transform and opacity animate, so the beat stays on the compositor. */
  .agent-spinner {
    --color-1: var(--primary);
    /* A lighter shade of the same hue, so the beat reads as a glow, not a flash. */
    --color-2: color-mix(in oklch, var(--color-1) 70%, white);
    --size: 0.5px;

    position: relative;
    display: inline-block;
    flex-shrink: 0;
    width: calc(24 * var(--size));
    height: calc(24 * var(--size));
    border-radius: 50%;
    background: var(--color-1);
    animation: agent-heartbeat var(--agent-beat-duration) ease-in-out infinite;
  }

  .agent-spinner::before,
  .agent-spinner::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: 50%;
    animation: var(--agent-beat-duration) ease-in-out infinite;
    pointer-events: none;
  }

  .agent-spinner::before {
    background: var(--color-2);
    opacity: 0;
    animation-name: agent-heartbeat-glow;
  }

  .agent-spinner::after {
    background: var(--color-1);
    opacity: 0;
    animation-name: agent-heartbeat-ring;
  }

  @keyframes agent-heartbeat {
    0%,
    20%,
    50%,
    100% {
      transform: scale(1);
    }
    10%,
    30% {
      transform: scale(1.35);
    }
  }

  @keyframes agent-heartbeat-glow {
    0%,
    20%,
    60%,
    100% {
      opacity: 0;
    }
    10%,
    30% {
      opacity: 1;
    }
  }

  @keyframes agent-heartbeat-ring {
    0% {
      transform: scale(1);
      opacity: 0.4;
    }
    50%,
    100% {
      /* Out to the old 20-unit box-shadow spread around the 24-unit dot. */
      transform: scale(2.67);
      opacity: 0;
    }
  }
</style>
