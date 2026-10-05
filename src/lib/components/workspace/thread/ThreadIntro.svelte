<script lang="ts">
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import Buddy from '$lib/components/buddy/Buddy.svelte'
  import { agentName } from '$lib/agent-name'
  import { buddyColor, buddyMoodForThread } from '$lib/buddy'
  import { engineDisplayName } from '$lib/sidebar/engine'
  let {
    thread,
    workspace,
    otherThreadCount,
    nowMs,
  }: {
    thread: Thread
    workspace: Workspace
    otherThreadCount: number
    nowMs: number
  } = $props()

  const isLead = $derived(thread.role === 'Lead')
  const mood = $derived(buddyMoodForThread(thread))
  const color = $derived(buddyColor(thread.id))
  const name = $derived(agentName(thread.id))
  const engineName = $derived(engineDisplayName(thread.engine))

  const relationship = $derived.by(() => {
    if (isLead) {
      if (otherThreadCount > 0) {
        return `leads ${otherThreadCount} other ${otherThreadCount === 1 ? 'thread' : 'threads'} in this worktree`
      }
      return 'owns this worktree'
    }
    return 'shares the worktree with Lead'
  })

  const summaryAge = $derived.by(() => {
    if (!workspace.summaryAtMs) return null
    return formatRelativeAge(workspace.summaryAtMs, nowMs)
  })
</script>

<header class="thread-intro space-y-4 pb-2" data-od-id="thread-intro">
  <div class="flex items-start gap-3">
    <Buddy {mood} {color} glasses={thread.role === 'Reviewer'} size={36} />
    <div class="min-w-0 space-y-1">
      <h2 class="text-lg font-semibold tracking-tight">{name}</h2>
      <p class="text-sm text-muted-foreground">
        {engineName} · {thread.role} · {relationship}
      </p>
    </div>
  </div>

  {#if isLead && workspace.summary?.trim()}
    <div
      class="rounded-md border border-border/60 bg-muted/20 px-3 py-2 text-sm text-muted-foreground"
    >
      <p class="text-foreground/90">{workspace.summary}</p>
      {#if summaryAge}
        <p class="mt-1 text-xs">
          Summarized by {workspace.summarySource}, {summaryAge}
        </p>
      {/if}
    </div>
  {/if}
</header>
