<script lang="ts">
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import {
    plural,
    statusDotVariantForWorkspace,
    workspaceStatusWord,
  } from '$lib/sidebar/status'
  import { app, workspaceRecords } from '$lib/state'
  import type { ThreadListRowItem } from '$lib/state/sidebar-workspaces.svelte'
  import { cn } from '$lib/utils'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import StatusDot from './StatusDot.svelte'

  let { item, nowMs }: { item: ThreadListRowItem; nowMs: number } = $props()

  const workspace = $derived(item.workspace)
  const attention = $derived(item.urgency === 'attention')
  // In progress isn't your problem: a spinner, and everything else steps back.
  const working = $derived(workspace != null && !attention)

  const isCurrent = $derived(
    workspace
      ? app.view === 'workspace' && app.workspaceId === item.id
      : app.view === 'scratch' && app.scratchId === item.id,
  )

  const statusInput = $derived(
    workspace && {
      lifecycle: workspace.lifecycle,
      paused: workspace.paused,
      activityText: workspace.activityText,
    },
  )

  const statusLine = $derived.by(() => {
    if (!workspace || !statusInput) return 'Read-only'
    if (workspace.lifecycle === 'provisioningFailed') return 'Setup failed'
    if (workspace.pendingApprovals > 0) {
      return `${plural(workspace.pendingApprovals, 'approval', 'approvals')} waiting`
    }
    return workspaceStatusWord(statusInput)
  })

  // An approval can land while the workspace still reads as running, so urgency decides.
  const dotVariant = $derived.by(() => {
    if (attention) return 'waiting'
    return statusInput ? statusDotVariantForWorkspace(statusInput) : 'idle'
  })

  // Only what the dot can't say: a setup that failed, or a leftover that can't be prompted.
  const showStatus = $derived(
    !workspace || workspace.lifecycle === 'provisioningFailed',
  )

  // The workspace's dev app, shown so a running server is findable without opening it.
  const appRuntime = $derived(workspace && workspaceRecords.runtime(item.id))
  const appPort = $derived(
    appRuntime?.appStatus === 'running' ? appRuntime.port : null,
  )

  const age = $derived(
    item.activityAtMs ? formatRelativeAge(item.activityAtMs, nowMs) : null,
  )

  const ariaLabel = $derived(
    [
      item.title,
      item.repoName,
      item.branch,
      attention ? `needs attention, ${statusLine}` : statusLine,
      appPort ? `app running on port ${appPort}` : null,
      item.parentTitle ? `started by ${item.parentTitle}` : null,
    ]
      .filter(Boolean)
      .join(', '),
  )

  function open() {
    if (workspace) app.openWorkspace(item.id)
    else app.openScratch(item.id)
  }
</script>

<button
  type="button"
  class={cn(
    'thread-row relative grid w-full grid-cols-[16px_minmax(0,1fr)] gap-x-2 overflow-hidden rounded-lg px-2.5 py-2 text-left transition-[background-color,transform,box-shadow] duration-(--dur-fast) ease-(--ease-out-snappy) outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 active:scale-[0.98] aria-[current=page]:bg-sidebar-accent dark:hover:bg-muted/50',
    item.nested && 'pl-6',
  )}
  aria-current={isCurrent ? 'page' : undefined}
  aria-label={ariaLabel}
  data-od-id="thread-row-{item.id}"
  data-urgency={item.urgency}
  onclick={open}
>
  <span class="flex items-center">
    {#if working}
      <LoaderCircle
        class="size-3.5 animate-spin text-muted-foreground"
        data-od-id="thread-row-spinner"
      />
    {:else}
      <StatusDot variant={dotVariant} />
    {/if}
  </span>
  <span
    class={cn(
      'grid min-w-0 gap-0.5 transition-opacity duration-(--dur-med)',
      working && 'opacity-50',
    )}
    aria-hidden="true"
  >
    <span class="flex min-w-0 items-baseline gap-2">
      <span
        class={cn(
          'min-w-0 flex-1 truncate text-sm',
          working ? 'text-muted-foreground' : 'text-sidebar-foreground',
        )}>{item.title}</span
      >
      {#if age}
        <span
          class="shrink-0 font-mono text-[10px] text-muted-foreground/80 tabular-nums"
          >{age}</span
        >
      {/if}
    </span>
    {#if item.repoName || item.branch}
      <span
        class="flex min-w-0 items-center gap-1 text-[11px] text-muted-foreground"
      >
        {#if item.repoName}
          <span class="max-w-[55%] shrink-0 truncate">{item.repoName}</span>
        {/if}
        {#if item.repoName && item.branch}
          <span class="opacity-60">·</span>
        {/if}
        {#if item.branch}
          <span class="min-w-0 truncate font-mono text-[10px]"
            >{item.branch}</span
          >
        {/if}
        {#if appPort}
          <span
            class="ml-auto flex shrink-0 items-center gap-1 font-mono text-[10px] text-emerald-600"
            title="App running on localhost:{appPort}"
          >
            <span class="size-1.5 rounded-full bg-emerald-500"></span>:{appPort}
          </span>
        {/if}
      </span>
    {/if}
    {#if item.parentTitle && !item.nested}
      <span class="truncate text-[11px] text-muted-foreground">
        Started by {item.parentTitle}
      </span>
    {/if}
    {#if showStatus}
      <span
        class={cn(
          'flex min-w-0 items-center gap-1 text-xs text-muted-foreground',
        )}
      >
        <span class="truncate">{statusLine}</span>
      </span>
    {/if}
  </span>
</button>
