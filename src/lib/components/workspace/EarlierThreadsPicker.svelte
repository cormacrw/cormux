<script lang="ts">
  import * as Select from '$lib/components/ui/select'
  import { agentName } from '$lib/agent-name'
  import { app, threads } from '$lib/state'
  import History from '@lucide/svelte/icons/history'

  // Threads from before a workspace had just one. They're kept to read; most workspaces have none.
  let { workspaceId }: { workspaceId: string } = $props()

  const earlier = $derived(threads.retiredFor(workspaceId))
  const live = $derived(threads.liveFor(workspaceId))
</script>

{#if earlier.length}
  <Select.Root
    type="single"
    value={app.threadId ?? ''}
    onValueChange={(value) => {
      if (value) app.threadId = value
    }}
  >
    <Select.Trigger
      size="sm"
      class="glass-well h-7 shrink-0 gap-1.5 rounded-[9px] border-transparent text-xs"
      aria-label="Earlier threads"
      data-od-id="earlier-threads"
    >
      <History class="size-3.5 opacity-70" aria-hidden="true" />
      Earlier threads
    </Select.Trigger>
    <Select.Content>
      {#if live}
        <Select.Item value={live.id} label={agentName(live.id)}>
          {agentName(live.id)} (current)
        </Select.Item>
      {/if}
      {#each earlier as thread (thread.id)}
        <Select.Item value={thread.id} label={agentName(thread.id)}>
          {agentName(thread.id)}
        </Select.Item>
      {/each}
    </Select.Content>
  </Select.Root>
{/if}
