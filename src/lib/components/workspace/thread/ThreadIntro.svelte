<script lang="ts">
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { engineDisplayName } from '$lib/engine/engines'
  let {
    thread,
    workspace,
  }: {
    thread: Thread
    workspace: Workspace
  } = $props()

  const isLead = $derived(thread.ownRole === 'lead')
  const engineName = $derived(engineDisplayName(thread.engine))
</script>

<header class="thread-intro space-y-4 pb-2" data-od-id="thread-intro">
  <div class="min-w-0 space-y-1">
    <h2 class="text-lg font-semibold tracking-tight">{thread.role}</h2>
    <p class="text-sm text-muted-foreground">{engineName}</p>
  </div>

  {#if isLead && workspace.summary?.trim()}
    <div
      class="rounded-md border border-border/60 bg-muted/20 px-3 py-2 text-sm text-muted-foreground"
    >
      <p class="text-foreground/90">{workspace.summary}</p>
    </div>
  {/if}
</header>
