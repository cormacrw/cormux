<script lang="ts">
  import StatusDot from '$lib/components/shell/StatusDot.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import { requestNewThread } from '$lib/command-palette/actions'
  import { reviewReady } from '$lib/review/workspace-settings'
  import {
    app,
    findings,
    settings,
    threads,
    workspaceDiff,
    workspaceRecords,
    workspaceUi,
  } from '$lib/state'
  import { formatChangeCounts } from '$lib/workspace/diff-totals'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { statusDotVariantForThread } from '$lib/sidebar/status'
  import { cn } from '$lib/utils'
  import { isWorkspaceProvisioning } from '$lib/workspace/provisioning'
  import {
    activeThreadBarTabKey,
    buildThreadBarTabOrder,
    changesTabAriaLabel,
    findingsTabAriaLabel,
    moveTabFocusIndex,
    outputTabAriaLabel,
    showFindingsTab,
    tabKeyId,
    threadTabAriaLabel,
    type ThreadBarTabKey,
  } from '$lib/workspace/thread-tabs'
  import File from '@lucide/svelte/icons/file'
  import List from '@lucide/svelte/icons/list'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import Plus from '@lucide/svelte/icons/plus'
  import Terminal from '@lucide/svelte/icons/terminal'

  let { workspace }: { workspace: Workspace } = $props()

  let barEl: HTMLDivElement | undefined = $state()

  const wsThreads = $derived(threads.forWorkspace(workspace.id))
  const findingCount = $derived(findings.forWorkspace(workspace.id).length)
  const showFindings = $derived(
    showFindingsTab({
      workspaceKind: workspace.kind,
      reviewReady: reviewReady(settings.rows, workspace.id),
    }),
  )
  const openFindings = $derived(findings.openCount(workspace.id))
  const tabOrder = $derived(buildThreadBarTabOrder(wsThreads, showFindings))
  const provisioning = $derived(isWorkspaceProvisioning(workspace.lifecycle))
  const runtime = $derived(workspaceRecords.runtime(workspace.id))
  const changeCountLabel = $derived(formatChangeCounts(workspaceDiff.totals(workspace.id)))

  function isSelected(tab: ThreadBarTabKey): boolean {
    const active = activeThreadBarTabKey({
      tabs: tabOrder,
      panelTab: workspaceUi.activeTab,
      threadId: app.threadId,
    })
    if (!active) return false
    return tabKeyId(tab) === tabKeyId(active)
  }

  function tabIndex(tab: ThreadBarTabKey): number {
    return isSelected(tab) ? 0 : -1
  }

  function selectTab(tab: ThreadBarTabKey) {
    if (tab.kind === 'thread') {
      app.threadId = tab.threadId
      workspaceUi.openTab('thread')
      return
    }
    workspaceUi.openTab(tab.kind)
  }

  function onBarKeydown(event: KeyboardEvent) {
    const keys = ['ArrowLeft', 'ArrowRight', 'Home', 'End'] as const
    if (!keys.includes(event.key as (typeof keys)[number])) return
    const tabs = barEl?.querySelectorAll<HTMLElement>('[role="tab"]')
    if (!tabs?.length) return
    const focused = document.activeElement
    const current = [...tabs].indexOf(focused as HTMLElement)
    if (current < 0) return
    event.preventDefault()
    const nextIndex = moveTabFocusIndex(
      current,
      tabs.length,
      event.key as (typeof keys)[number],
    )
    const next = tabs[nextIndex]
    const key = tabOrder[nextIndex]
    if (key) selectTab(key)
    next?.focus()
  }

  function threadTabClass(selected: boolean) {
    return cn(
      'h-8 max-w-[11rem] shrink-0 gap-1.5 rounded-t-md rounded-b-none border border-transparent px-2.5 font-normal shadow-none',
      selected
        ? '-mb-px border-border/70 border-b-background bg-background text-foreground'
        : 'text-muted-foreground hover:bg-muted/40 hover:text-foreground',
    )
  }
</script>

<div
  bind:this={barEl}
  role="group"
  aria-label="Thread tabs"
  class="thread-bar flex h-9 shrink-0 items-end gap-1 overflow-x-auto border-b border-border/60 px-3 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
  data-od-id="thread-tabs"
  onkeydown={onBarKeydown}
>
  <div
    id="thread-tabs"
    class="flex min-w-0 items-end gap-0.5"
    role="tablist"
    aria-label="Agent threads"
  >
    {#each wsThreads as thread (thread.id)}
      {@const selected =
        workspaceUi.activeTab === 'thread' && app.threadId === thread.id}
      {@const statusInput = {
        status: thread.status,
        paused: thread.paused,
        activity: thread.activity,
      }}
      <Button
        id="thread-tab-{thread.id}"
        role="tab"
        variant="ghost"
        size="sm"
        class={threadTabClass(selected)}
        aria-selected={selected}
        aria-controls="thread-panel"
        tabindex={tabIndex({ kind: 'thread', threadId: thread.id })}
        aria-label={threadTabAriaLabel(thread)}
        data-od-id="thread-tab-{thread.id}"
        onclick={() => selectTab({ kind: 'thread', threadId: thread.id })}
      >
        <StatusDot variant={statusDotVariantForThread(statusInput)} />
        <span class="truncate" aria-hidden="true">{thread.role}</span>
        {#if thread.pendingApprovals > 0}
          <Badge
            variant="outline"
            class="min-w-[18px] justify-center border-warning/40 bg-warning/15 px-1 font-mono text-[10px] text-warning"
            aria-hidden="true"
          >
            {thread.pendingApprovals}
          </Badge>
        {/if}
      </Button>
    {/each}

    {#if showFindings}
      {@const selected = workspaceUi.activeTab === 'findings'}
      <Button
        id="thread-tab-findings"
        role="tab"
        variant="ghost"
        size="sm"
        class={threadTabClass(selected)}
        aria-selected={selected}
        aria-controls="findings-panel"
        tabindex={tabIndex({ kind: 'findings' })}
        aria-label={findingsTabAriaLabel(openFindings)}
        data-od-id="thread-tab-findings"
        onclick={() => selectTab({ kind: 'findings' })}
      >
        <List class="size-3.5 shrink-0" aria-hidden="true" />
        <span class="truncate" aria-hidden="true">Findings</span>
        {#if openFindings > 0}
          <Badge
            variant="secondary"
            class="min-w-[18px] justify-center px-1 font-mono text-[10px]"
            aria-hidden="true"
          >
            {openFindings}
          </Badge>
        {/if}
      </Button>
    {/if}

    <Button
      id="thread-add"
      variant="ghost"
      size="icon-sm"
      class="mb-px size-7 shrink-0 text-muted-foreground"
      aria-label="New thread"
      data-od-id="thread-add"
      onclick={() => requestNewThread(workspace.id)}
    >
      <Plus class="size-4" aria-hidden="true" />
    </Button>
  </div>

  <span class="min-w-2 flex-1" aria-hidden="true"></span>

  <div
    id="out-tabs"
    role="tablist"
    aria-label="Changes and app"
    class="flex shrink-0 items-end gap-0.5"
  >
    <Button
      id="thread-tab-changes"
      role="tab"
      variant="ghost"
      size="sm"
      class={threadTabClass(workspaceUi.activeTab === 'changes')}
      aria-selected={workspaceUi.activeTab === 'changes'}
      aria-controls="changes-panel"
      tabindex={tabIndex({ kind: 'changes' })}
      aria-label={changesTabAriaLabel(changeCountLabel)}
      data-od-id="thread-tab-changes"
      onclick={() => selectTab({ kind: 'changes' })}
    >
      <File class="size-3.5 shrink-0" aria-hidden="true" />
      <span class="truncate" aria-hidden="true">Changes</span>
      {#if changeCountLabel}
        <span class="font-mono text-[10px] text-muted-foreground" aria-hidden="true">
          {changeCountLabel}
        </span>
      {/if}
    </Button>
    <Button
      id="thread-tab-output"
      role="tab"
      variant="ghost"
      size="sm"
      class={threadTabClass(workspaceUi.activeTab === 'output')}
      aria-selected={workspaceUi.activeTab === 'output'}
      aria-controls="output-panel"
      tabindex={tabIndex({ kind: 'output' })}
      aria-label={outputTabAriaLabel({
        provisioning,
        appStatus: runtime.appStatus,
        port: runtime.port,
      })}
      data-od-id="thread-tab-output"
      onclick={() => selectTab({ kind: 'output' })}
    >
      <Terminal class="size-3.5 shrink-0" aria-hidden="true" />
      <span class="truncate" aria-hidden="true">Output</span>
      {#if provisioning || runtime.appStatus === 'starting'}
        <LoaderCircle class="size-3.5 animate-spin" aria-hidden="true" />
      {:else if runtime.appStatus === 'running' && runtime.port}
        <span class="flex items-center gap-1" aria-hidden="true">
          <span class="size-2 rounded-full bg-emerald-500"></span>
          <span class="font-mono text-[10px] text-muted-foreground"
            >:{runtime.port}</span
          >
        </span>
      {/if}
    </Button>
  </div>
</div>
