<!-- The agent at work, as the last row of a conversation without a prompt bar to float it over. -->
<script lang="ts">
  import Pause from '@lucide/svelte/icons/pause'
  import type { LiveStatus } from '$lib/thread/live-status'
  import AgentSpinner from '../AgentSpinner.svelte'

  let { live }: { live: LiveStatus } = $props()
</script>

<li class="py-1">
  <div class="flex min-w-0 items-center gap-2 py-0.5 text-xs">
    {#if live.paused}
      <Pause class="size-3 shrink-0 text-muted-foreground" aria-hidden="true" />
    {:else}
      <AgentSpinner class="mr-2" />
    {/if}
    <span
      class={[
        'shrink-0 font-medium text-foreground/80',
        // Re-added on resume as the spinner remounts, so both restart the beat together.
        !live.paused && 'agent-working-text',
      ]}>{live.title}</span
    >
    {#if live.subtitle && live.subtitle !== live.title}
      <span class="truncate text-muted-foreground">· {live.subtitle}</span>
    {/if}
  </div>
</li>
