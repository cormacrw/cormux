<script lang="ts">
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { engineDisplayName, engineMark } from '$lib/sidebar/engine'
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
  const mark = $derived(engineMark(thread.engine))
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
    <span
      class="flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/40 font-mono text-xs font-semibold text-muted-foreground"
      aria-hidden="true">{mark}</span
    >
    <div class="min-w-0 space-y-1">
      <h2 class="text-lg font-semibold tracking-tight">{thread.role}</h2>
      <p class="text-sm text-muted-foreground">
        {engineName} · {relationship}
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
