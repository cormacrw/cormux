<script lang="ts">
  import { tick, untrack } from 'svelte'
  import ArrowRight from '@lucide/svelte/icons/arrow-right'
  import Check from '@lucide/svelte/icons/check'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { app, findings, threads, workspaceUi, workspaces } from '$lib/state'
  import { agentName } from '$lib/agent-name'
  import { engineDisplayName } from '$lib/sidebar/engine'
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
  import type { FindingRow } from '$lib/ipc/bindings'
  import { cn } from '$lib/utils'

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

  // Re-run when the findings or threads change. The store calls read and write the
  // selection, so running them tracked would re-trigger this effect forever.
  $effect(() => {
    void rows
    const threadIds = wsThreads.map((thread) => thread.id)
    untrack(() => {
      findings.seedDefaults(workspaceId)
      findings.ensureTargetThread(workspaceId, threadIds)
    })
  })

  const targetThreadId = $derived(
    findings.targetThreadId[workspaceId] ?? wsThreads[0]?.id ?? '',
  )
  const targetThread = $derived(
    wsThreads.find((thread) => thread.id === targetThreadId) ?? wsThreads[0],
  )

  const engineName = $derived(engineDisplayName(reviewer?.engine ?? 'claude'))
  const prLabel = $derived(
    workspace?.prNumber != null ? `#${workspace.prNumber}` : 'This PR',
  )

  function threadRoleForFinding(row: FindingRow) {
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

  function openLocation(row: FindingRow) {
    if (!row.file) return
    workspaceUi.revealInChanges(row.file, row.line)
  }

  async function sendFindings() {
    if (!targetThread || sending) return
    sending = true
    sendError = null
    try {
      await findings.sendSelected(workspaceId, targetThread.id)
    } catch (error) {
      sendError =
        error instanceof Error ? error.message : 'Could not send findings'
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
    void workspaceUi.activeTab
    queueMicrotask(() => {
      if (bodyEl) bodyEl.scrollTop = 0
    })
  })
</script>

<div
  id="findings-panel"
  role="tabpanel"
  aria-labelledby="thread-tab-findings"
  data-od-id="findings"
  class="flex min-h-0 flex-1 flex-col"
>
  <div
    bind:this={bodyEl}
    id="findings-body"
    class="min-h-0 flex-1 overflow-y-auto"
  >
    <div class="mx-auto w-full max-w-3xl px-8 pt-8 pb-12 select-text">
      <header class="space-y-5" data-od-id="findings-intro">
        <div class="space-y-1">
          <h2
            bind:this={headingRef}
            id="fnd-heading"
            tabindex="-1"
            class="text-xl font-semibold tracking-tight outline-none"
          >
            Review findings
          </h2>
          <p class="text-sm text-muted-foreground">
            {prLabel} · reviewed by {reviewer
              ? agentName(reviewer.id)
              : 'Reviewer'} with {engineName}
          </p>
        </div>

        {#if rows.length}
          <div
            class="flex flex-wrap items-center gap-x-5 gap-y-3 border-b border-border/60 pb-5 text-sm"
          >
            {#each FINDING_GROUPS as group (group.severity)}
              {@const count = rows.filter(
                (row) => row.severity === group.severity,
              ).length}
              {#if count}
                <span
                  class="inline-flex items-center gap-2 text-muted-foreground"
                >
                  <span
                    class="size-2 rounded-full {severityDotClass(
                      group.severity,
                    )}"
                    aria-hidden="true"
                  ></span>
                  <span
                    ><strong class="font-semibold text-foreground"
                      >{count}</strong
                    >
                    {group.label.toLowerCase()}</span
                  >
                </span>
              {/if}
            {/each}
            <span class="grow"></span>
            <div
              class="flex items-center gap-1 select-none"
              role="group"
              aria-label="Quick select"
            >
              <span class="mr-1 text-xs text-muted-foreground">Select</span>
              <Button
                variant="ghost"
                size="xs"
                disabled={!openRows.length}
                onclick={() =>
                  preserveScroll(() =>
                    findings.quickSelect(workspaceId, 'blocking'),
                  )}
              >
                Blocking
              </Button>
              <Button
                variant="ghost"
                size="xs"
                disabled={!openRows.length}
                onclick={() =>
                  preserveScroll(() =>
                    findings.quickSelect(workspaceId, 'all'),
                  )}
              >
                All
              </Button>
              <Button
                variant="ghost"
                size="xs"
                disabled={!summary.total}
                onclick={() =>
                  preserveScroll(() =>
                    findings.quickSelect(workspaceId, 'none'),
                  )}
              >
                None
              </Button>
            </div>
          </div>
        {/if}
      </header>

      {#if !rows.length}
        <div class="py-16 text-center" data-od-id="findings-empty">
          <h3 class="text-base font-medium">Nothing to fix</h3>
          <p class="mt-1 text-sm text-muted-foreground">
            {reviewer?.role ?? 'Reviewer'} didn’t flag anything in this pull request.
          </p>
        </div>
      {:else}
        <div class="mt-8 space-y-10">
          {#each FINDING_GROUPS as group (group.severity)}
            {@const groupRows = rows.filter(
              (row) => row.severity === group.severity,
            )}
            {#if groupRows.length}
              {@const box = groupCheckboxState(rows, selected, group.severity)}
              <section
                class="space-y-3"
                aria-labelledby="fnd-h-{group.severity}"
                data-od-id="findings-{group.severity}"
              >
                <div class="flex items-center gap-3 select-none">
                  <Checkbox
                    checked={box.checked}
                    indeterminate={box.indeterminate}
                    disabled={box.disabled}
                    aria-label="Select all open {group.label.toLowerCase()}"
                    onCheckedChange={(value) =>
                      preserveScroll(() =>
                        findings.setGroup(
                          workspaceId,
                          group.severity,
                          value === true,
                        ),
                      )}
                  />
                  <span
                    class="size-2 rounded-full {severityDotClass(
                      group.severity,
                    )}"
                    aria-hidden="true"
                  ></span>
                  <h3 id="fnd-h-{group.severity}" class="text-sm font-semibold">
                    {group.label}
                    <span class="ml-1 font-normal text-muted-foreground"
                      >{groupRows.length}</span
                    >
                  </h3>
                  <span class="text-xs text-muted-foreground">{group.note}</span
                  >
                </div>

                <ul class="space-y-3">
                  {#each groupRows as row (row.id)}
                    {@const boxState = rowCheckboxState(row, selected)}
                    {@const done = row.status !== 'open'}
                    <li
                      data-od-id="finding-{row.id}"
                      class={cn(
                        'grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 rounded-lg border border-border/70 px-5 py-4 transition-colors',
                        boxState.checked &&
                          !done &&
                          'border-primary/40 bg-primary/5',
                        done && 'opacity-60',
                      )}
                    >
                      <Checkbox
                        class="mt-0.5"
                        checked={boxState.checked}
                        disabled={boxState.disabled}
                        aria-label="Select “{row.title}”"
                        aria-describedby="fnd-b-{row.id}"
                        onCheckedChange={(value) =>
                          preserveScroll(() =>
                            findings.toggle(
                              workspaceId,
                              row.id,
                              value === true,
                            ),
                          )}
                      />
                      <div class="min-w-0">
                        <div class="flex items-start justify-between gap-3">
                          <h4 class="text-sm leading-snug font-medium">
                            {row.title}
                          </h4>
                          {#if row.status === 'fixed'}
                            <span
                              class="inline-flex shrink-0 items-center gap-1 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-xs text-emerald-700 select-none dark:text-emerald-300"
                            >
                              <Check class="size-3" aria-hidden="true" />
                              Fixed by {threadRoleForFinding(row)}
                            </span>
                          {:else if row.status === 'sent'}
                            <span
                              class="inline-flex shrink-0 items-center gap-1 rounded-full border border-primary/30 bg-primary/10 px-2 py-0.5 text-xs text-primary select-none"
                            >
                              <LoaderCircle
                                class="size-3 animate-spin"
                                aria-hidden="true"
                              />
                              Sent to {threadRoleForFinding(row)}
                            </span>
                          {/if}
                        </div>
                        {#if row.file}
                          <button
                            type="button"
                            class="mt-1 max-w-full truncate rounded-sm text-left font-mono text-xs text-muted-foreground underline decoration-dotted underline-offset-4 hover:text-foreground hover:decoration-solid"
                            title="Open in Changes"
                            data-od-id="finding-location"
                            onclick={() => openLocation(row)}
                          >
                            {findingLocation(row)}
                          </button>
                        {/if}
                        {#if row.explanation}
                          <p
                            id="fnd-b-{row.id}"
                            class="mt-2 text-sm leading-relaxed whitespace-pre-wrap text-muted-foreground"
                          >
                            {row.explanation}
                          </p>
                        {/if}
                      </div>
                    </li>
                  {/each}
                </ul>
              </section>
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <div
    id="findings-bar"
    data-od-id="findings-send-bar"
    class="border-t border-border/60 bg-background"
  >
    <div
      class="mx-auto flex w-full max-w-3xl flex-wrap items-center gap-3 px-8 py-3"
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
                <option value={thread.id}>{agentName(thread.id)}</option>
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
            Send {summary.total} to {agentName(targetThread.id)}
          {:else}
            Send to agent
          {/if}
        </Button>
      {/if}
    </div>
  </div>
</div>
