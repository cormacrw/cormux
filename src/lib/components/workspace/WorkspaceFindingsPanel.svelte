<script lang="ts">
  import { tick } from 'svelte'
  import ArrowRight from '@lucide/svelte/icons/arrow-right'
  import Check from '@lucide/svelte/icons/check'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import {
    app,
    findings,
    threads,
    workspaceUi,
    workspaces,
  } from '$lib/state'
  import { engineDisplayName, engineMark } from '$lib/sidebar/engine'
  import { selectionSummaryText } from '$lib/findings/format'
  import {
    groupCheckboxState,
    rowCheckboxState,
    selectedSummary,
  } from '$lib/findings/selection'
  import {
    FINDING_GROUPS,
    findingLocation,
    openFindings,
  } from '$lib/findings/types'
  import { FINDINGS_KNOWN_GAPS } from '$lib/findings/known-gaps'

  let {
    workspaceId,
    headingRef = $bindable(),
  }: {
    workspaceId: string
    headingRef?: HTMLHeadingElement | undefined
  } = $props()

  let bodyEl: HTMLDivElement | undefined = $state()
  let sending = $state(false)
  let sendError = $state<string | null>(null)
  let focusFindingId = $state<string | null>(null)

  const workspace = $derived(workspaces.getById(workspaceId))
  const rows = $derived(findings.forWorkspace(workspaceId))
  const openRows = $derived(openFindings(rows))
  const wsThreads = $derived(threads.forWorkspace(workspaceId))
  const reviewer = $derived(
    wsThreads.find((thread) => thread.role === 'Reviewer') ?? wsThreads[0],
  )
  const selected = $derived(findings.selectedIds(workspaceId))
  const summary = $derived(selectedSummary(rows, selected))
  const barSummary = $derived(selectionSummaryText(summary))

  $effect(() => {
    findings.seedDefaults(workspaceId)
    findings.ensureTargetThread(
      workspaceId,
      wsThreads.map((thread) => thread.id),
    )
  })

  const targetThreadId = $derived(
    findings.targetThreadId[workspaceId] ?? wsThreads[0]?.id ?? '',
  )
  const targetThread = $derived(
    wsThreads.find((thread) => thread.id === targetThreadId) ?? wsThreads[0],
  )

  const engine = $derived(reviewer?.engine ?? 'claude')
  const mark = $derived(engineMark(engine))
  const engineName = $derived(engineDisplayName(engine))
  const prLabel = $derived(
    workspace?.prNumber != null ? `#${workspace.prNumber}` : 'this PR',
  )

  function threadRoleForFinding(row: (typeof rows)[number]) {
    if (!row.sentToThreadId) return targetThread?.role ?? 'agent'
    return threads.getById(row.sentToThreadId)?.role ?? 'agent'
  }

  function severityDotClass(severity: string) {
    if (severity === 'blocking') return 'bg-red-500'
    if (severity === 'suggestion') return 'bg-amber-400'
    return 'bg-muted-foreground/60'
  }

  async function preserveScroll(run: () => void | Promise<void>) {
    const top = bodyEl?.scrollTop ?? 0
    await run()
    await tick()
    if (bodyEl) bodyEl.scrollTop = top
  }

  async function onRowToggle(id: string, checked: boolean) {
    focusFindingId = id
    await preserveScroll(() => findings.toggle(workspaceId, id, checked))
  }

  async function onGroupToggle(severity: string, checked: boolean) {
    await preserveScroll(() =>
      findings.setGroup(workspaceId, severity, checked),
    )
  }

  async function onQuickSelect(mode: 'blocking' | 'all' | 'none') {
    await preserveScroll(() => findings.quickSelect(workspaceId, mode))
  }

  async function sendFindings() {
    if (!targetThread || sending) return
    sending = true
    sendError = null
    try {
      await findings.sendSelected(workspaceId, targetThread.id)
    } catch (error) {
      sendError = error instanceof Error ? error.message : 'Could not send findings'
    } finally {
      sending = false
    }
  }

  function goToTargetThread() {
    if (!targetThread) return
    app.threadId = targetThread.id
    workspaceUi.openTab('thread')
  }

  $effect(() => {
    void focusFindingId
    void rows
    queueMicrotask(() => {
      if (!focusFindingId) return
      const el = document.querySelector<HTMLElement>(
        `[data-finding-focus="${focusFindingId}"]`,
      )
      el?.focus()
    })
  })

  $effect(() => {
    void workspaceUi.activeTab
    queueMicrotask(() => {
      if (bodyEl) bodyEl.scrollTop = 0
    })
  })
</script>

<section
  id="findings-panel"
  role="tabpanel"
  aria-labelledby="thread-tab-findings"
  data-od-id="findings"
  class="flex min-h-0 flex-1 flex-col"
>
  <div bind:this={bodyEl} id="findings-body" class="min-h-0 flex-1 overflow-y-auto">
    <header
      class="space-y-4 border-b border-border/60 px-1 pb-4 pt-1"
      data-od-id="findings-intro"
    >
      <div class="flex items-start gap-3">
        <span
          class="flex size-9 shrink-0 items-center justify-center rounded-md border border-border/70 bg-muted/40 font-mono text-xs font-semibold text-muted-foreground"
          aria-hidden="true">{mark}</span
        >
        <div class="min-w-0 space-y-1">
          <h2
            bind:this={headingRef}
            id="fnd-heading"
            tabindex="-1"
            class="text-lg font-semibold tracking-tight outline-none"
          >
            Review findings
          </h2>
          <p class="text-sm text-muted-foreground">
            {prLabel} · reviewed by {reviewer?.role ?? 'Reviewer'} with {engineName}
          </p>
        </div>
      </div>

      {#if rows.length}
        <div class="flex flex-wrap items-center gap-3 text-sm">
          <div class="flex flex-wrap gap-3 text-muted-foreground">
            {#each FINDING_GROUPS as group (group.severity)}
              {@const count = rows.filter((row) => row.severity === group.severity).length}
              {#if count}
                <span class="inline-flex items-center gap-2">
                  <span
                    class="size-2 rounded-full {severityDotClass(group.severity)}"
                    aria-hidden="true"
                  ></span>
                  <span><strong class="text-foreground">{count}</strong>
                    {group.label.toLowerCase()}</span
                  >
                </span>
              {/if}
            {/each}
          </div>
          <span class="grow"></span>
          <div
            class="flex flex-wrap items-center gap-1"
            role="group"
            aria-label="Quick select"
          >
            <span class="text-xs text-muted-foreground">Select</span>
            <Button
              variant="ghost"
              size="sm"
              disabled={!openRows.length}
              onclick={() => onQuickSelect('blocking')}
            >
              Blocking
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={!openRows.length}
              onclick={() => onQuickSelect('all')}
            >
              All
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={!summary.total}
              onclick={() => onQuickSelect('none')}
            >
              None
            </Button>
          </div>
        </div>
      {/if}
    </header>

    {#if !rows.length}
      <div class="px-1 py-10 text-center" data-od-id="findings-empty">
        <h3 class="text-base font-medium">Nothing to fix</h3>
        <p class="mt-1 text-sm text-muted-foreground">
          {reviewer?.role ?? 'Reviewer'} didn’t flag anything in this pull request.
        </p>
      </div>
    {:else}
      <div class="fnd-groups space-y-6 px-1 py-4">
        {#each FINDING_GROUPS as group (group.severity)}
          {@const groupRows = rows.filter((row) => row.severity === group.severity)}
          {#if groupRows.length}
            {@const box = groupCheckboxState(rows, selected, group.severity)}
            <section
              class="space-y-2"
              aria-labelledby="fnd-h-{group.severity}"
              data-od-id="findings-{group.severity}"
            >
              <div class="flex flex-wrap items-center gap-2">
                <Checkbox
                  checked={box.checked}
                  indeterminate={box.indeterminate}
                  disabled={box.disabled}
                  aria-label="Select all open {group.label.toLowerCase()}"
                  onCheckedChange={(value) =>
                    onGroupToggle(group.severity, value === true)}
                />
                <span
                  class="size-2 rounded-full {severityDotClass(group.severity)}"
                  aria-hidden="true"
                ></span>
                <h3 id="fnd-h-{group.severity}" class="text-sm font-semibold">
                  {group.label}
                </h3>
                <span class="text-xs text-muted-foreground">{groupRows.length}</span>
                <span class="text-xs text-muted-foreground">{group.note}</span>
              </div>
              <ul class="space-y-2">
                {#each groupRows as row (row.id)}
                  {@const boxState = rowCheckboxState(row, selected)}
                  {@const done = row.status !== 'open'}
                  <li data-od-id="finding-{row.id}">
                    <label
                      class="flex cursor-pointer gap-3 rounded-md border border-border/70 px-3 py-2 transition-colors has-disabled:cursor-default {boxState.checked &&
                      !done
                        ? 'border-primary/30 bg-primary/5'
                        : ''} {done ? 'opacity-60' : ''}"
                    >
                      <Checkbox
                        data-finding-focus={row.id}
                        checked={boxState.checked}
                        disabled={boxState.disabled}
                        aria-describedby="fnd-b-{row.id}"
                        onCheckedChange={(value) =>
                          onRowToggle(row.id, value === true)}
                        onclick={(event) => event.stopPropagation()}
                      />
                      <span class="min-w-0 flex-1 space-y-1">
                        <span class="block text-sm font-medium">{row.title}</span>
                        {#if row.file}
                          <code class="text-xs text-muted-foreground"
                            >{findingLocation(row)}</code
                          >
                        {/if}
                        <p
                          id="fnd-b-{row.id}"
                          class="text-sm text-muted-foreground"
                        >
                          {row.explanation}
                        </p>
                      </span>
                      <span class="shrink-0 self-start pt-0.5">
                        {#if row.status === 'fixed'}
                          <span
                            class="inline-flex items-center gap-1 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-xs text-emerald-700 dark:text-emerald-300"
                          >
                            <Check class="size-3" aria-hidden="true" />
                            Fixed by {threadRoleForFinding(row)}
                          </span>
                        {:else if row.status === 'sent'}
                          <span
                            class="inline-flex items-center gap-1 rounded-full border border-primary/30 bg-primary/10 px-2 py-0.5 text-xs text-primary"
                          >
                            <LoaderCircle
                              class="size-3 animate-spin"
                              aria-hidden="true"
                            />
                            Sent to {threadRoleForFinding(row)}
                          </span>
                        {/if}
                      </span>
                    </label>
                  </li>
                {/each}
              </ul>
            </section>
          {/if}
        {/each}
      </div>

      <details class="mx-1 mb-4 rounded-md border border-border/60 px-3 py-2 text-xs text-muted-foreground">
        <summary class="cursor-pointer font-medium text-foreground/80">
          Known gaps (COR-188)
        </summary>
        <ul class="mt-2 list-disc space-y-1 pl-4">
          {#each FINDINGS_KNOWN_GAPS as gap (gap)}
            <li>{gap}</li>
          {/each}
        </ul>
      </details>
    {/if}
  </div>

  <div
    id="findings-bar"
    data-od-id="findings-send-bar"
    class="flex flex-wrap items-center gap-3 border-t border-border/60 bg-background px-3 py-3"
  >
    {#if !openRows.length}
      <p class="text-sm text-muted-foreground">
        {#if rows.length}
          Every finding has been sent.
          <span class="text-foreground/80"
            >Follow the fixes in the {targetThread?.role ?? 'agent'} thread.</span
          >
        {:else}
          No findings to send.
        {/if}
      </p>
      {#if rows.length && targetThread}
        <span class="grow"></span>
        <Button variant="secondary" onclick={goToTargetThread}>
          Go to {targetThread.role}
          <ArrowRight class="size-4" aria-hidden="true" />
        </Button>
      {/if}
    {:else}
      <p class="text-sm text-muted-foreground" aria-live="polite">
        {barSummary}
      </p>
      <span class="grow"></span>
      {#if wsThreads.length > 1}
        <label class="flex items-center gap-2 text-sm">
          <span class="text-muted-foreground">Send to</span>
          <select
            class="h-8 rounded-md border border-input bg-background px-2 text-sm"
            data-od-id="findings-target"
            value={targetThreadId}
            onchange={(event) => {
              const value = (event.currentTarget as HTMLSelectElement).value
              findings.setTargetThread(workspaceId, value)
            }}
          >
            {#each wsThreads as thread (thread.id)}
              <option value={thread.id}>{thread.role}</option>
            {/each}
          </select>
        </label>
      {/if}
      {#if sendError}
        <p class="text-xs text-destructive">{sendError}</p>
      {/if}
      <Button
        data-od-id="findings-send"
        disabled={!summary.total || sending || !targetThread}
        onclick={sendFindings}
      >
        <ArrowRight class="size-4" aria-hidden="true" />
        {#if summary.total && targetThread}
          Send {summary.total} to {targetThread.role}
        {:else}
          Send to agent
        {/if}
      </Button>
    {/if}
  </div>
</section>
