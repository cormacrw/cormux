<script lang="ts">
  import { buddyFace, clayColor } from '$lib/clay/identity'
  import Buddy from '$lib/components/clay/Buddy.svelte'
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import { statusDotVariantForThread } from '$lib/sidebar/status'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
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
  const engineName = $derived(engineDisplayName(thread.engine))
  const face = $derived(
    buddyFace(
      statusDotVariantForThread({
        status: thread.status,
        paused: thread.paused,
        activity: thread.activity,
      }),
      thread.pendingApprovals > 0,
    ),
  )

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

<header class="thread-intro flex flex-col items-center gap-3 pb-2 text-center" data-od-id="thread-intro">
  <div class="flex flex-col items-center gap-1">
    <Buddy
      color={clayColor(workspace.id)}
      {face}
      size={40}
      breathe={thread.status === 'running'}
    />
    <h2 class="font-display text-xl leading-none font-extrabold">{thread.role}</h2>
    <p class="text-sm text-muted-foreground">
      {engineName} · {relationship}
    </p>
  </div>

  {#if isLead && workspace.summary?.trim()}
    <div
      class="felt-sm w-full px-4 py-3 text-left text-sm text-muted-foreground"
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
