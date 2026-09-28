<script lang="ts">
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import * as Card from '$lib/components/ui/card'
  import type { FindingRow } from '$lib/ipc/bindings'
  import { engineMark } from '$lib/sidebar/engine'
  import ThoughtMarkdown from './ThoughtMarkdown.svelte'
  import {
    formatThreadTime,
    formatThreadTimeTitle,
    seqToApproxMs,
  } from '$lib/thread/thread-time'
  import type { TimelineRow, ToolRunStep } from '$lib/thread/timeline-types'
  import { workspaceDiff } from '$lib/state/workspace-diff.svelte'
  import { cn } from '$lib/utils'
  import { openDiffForPath } from './open-changes'
  import ApprovalCard from './ApprovalCard.svelte'
  import ArrowRight from '@lucide/svelte/icons/arrow-right'
  import Check from '@lucide/svelte/icons/check'
  import Copy from '@lucide/svelte/icons/copy'
  import FileText from '@lucide/svelte/icons/file-text'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import List from '@lucide/svelte/icons/list'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import Pause from '@lucide/svelte/icons/pause'
  import Play from '@lucide/svelte/icons/play'
  import Pencil from '@lucide/svelte/icons/pencil'
  import Search from '@lucide/svelte/icons/search'
  import Terminal from '@lucide/svelte/icons/terminal'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  let {
    rows,
    engine,
    workspaceId,
    findings,
    nowMs,
    timeBaseMs,
    liveTitle,
    liveSubtitle,
    paused,
    isNewIds = {},
    onOpenFindings,
    onFocusComposer,
  }: {
    rows: TimelineRow[]
    engine: string
    workspaceId: string
    findings: FindingRow[]
    nowMs: number
    timeBaseMs: number
    liveTitle: string
    liveSubtitle: string
    paused: boolean
    isNewIds: Record<string, true>
    onOpenFindings: () => void
    onFocusComposer?: () => void
  } = $props()

  const mark = $derived(engineMark(engine))
  const diffFiles = $derived(workspaceDiff.filesByWorkspace[workspaceId] ?? [])

  function timeLabel(seq: number) {
    const ms = seqToApproxMs(timeBaseMs, seq)
    return formatThreadTime(ms, nowMs)
  }

  function timeTitle(seq: number) {
    return formatThreadTimeTitle(seqToApproxMs(timeBaseMs, seq))
  }

  function diffCounts(path: string) {
    const file = diffFiles.find((row) => row.path === path)
    return { added: file?.added ?? 0, deleted: file?.deleted ?? 0 }
  }

  function stepIcon(step: ToolRunStep) {
    if (step.kind === 'edit') {
      return step.verb === 'Deleted' ? Trash2 : Pencil
    }
    const map = {
      file: FileText,
      tool: Terminal,
      branch: GitBranch,
      check: Check,
      terminal: Terminal,
      database: Terminal,
      pr: GitBranch,
      download: ArrowRight,
      list: List,
      search: Search,
      pencil: Pencil,
      trash: Trash2,
      pause: Pause,
      play: Play,
    } as const
    return map[step.icon] ?? Terminal
  }

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text)
    } catch {
      /* ignore */
    }
  }

  function findingSummary(items: FindingRow[]) {
    const counts: Record<string, number> = {}
    for (const row of items) {
      counts[row.severity] = (counts[row.severity] ?? 0) + 1
    }
    return Object.entries(counts)
      .map(([severity, count]) => `${count} ${severity}`)
      .join(' · ')
  }
</script>

{#each rows as row (row.kind === 'speaker' ? row.id : row.item.id)}
  {#if row.kind === 'speaker'}
    <li
      class="flex items-center gap-2 py-1 text-xs text-muted-foreground"
      aria-hidden="true"
    >
      {#if row.speaker === 'agent'}
        <span
          class="flex size-5 items-center justify-center rounded bg-muted font-mono text-[9px] font-semibold"
          >{mark}</span
        >
        <span class="font-medium text-foreground">{row.role}</span>
        <span>{engine}</span>
      {:else}
        <span class="font-medium text-foreground">You</span>
      {/if}
      <span class="text-muted-foreground/80" title={timeTitle(row.seq)}
        >· {timeLabel(row.seq)}</span
      >
    </li>
  {:else}
    {@const item = row.item}
    {@const isNew = Boolean(isNewIds[item.id])}
    <li
      class={cn(
        'py-1',
        isNew && 'animate-in fade-in duration-300',
        item.kind === 'user' && 'flex flex-col items-end gap-1',
      )}
    >
      {#if item.kind === 'user'}
        <div
          class="group max-w-[92%] rounded-2xl rounded-br-md bg-primary px-3 py-2 text-sm text-primary-foreground"
        >
          <p class="whitespace-pre-wrap">{item.text}</p>
          <button
            type="button"
            class="mt-1 inline-flex items-center gap-1 text-[10px] opacity-0 transition group-hover:opacity-70"
            onclick={() => copyText(item.text)}
          >
            <Copy class="size-3" aria-hidden="true" />
            Copy
          </button>
        </div>
        <span class="text-xs text-muted-foreground" title={timeTitle(item.seq)}
          >You · {timeLabel(item.seq)}</span
        >
      {:else if item.kind === 'thought'}
        <div class="group space-y-1 text-sm leading-relaxed text-foreground/90">
          <ThoughtMarkdown text={item.text} />
          <button
            type="button"
            class="inline-flex items-center gap-1 text-[10px] text-muted-foreground opacity-0 transition group-hover:opacity-100"
            onclick={() => copyText(item.text)}
          >
            <Copy class="size-3" aria-hidden="true" />
            Copy
          </button>
        </div>
      {:else if item.kind === 'toolRun'}
        <Card.Root class="overflow-hidden py-0">
          <Card.Content class="p-0">
            <ul class="divide-y divide-border/60">
              {#each item.steps as step (step.id)}
                {@const Icon = stepIcon(step)}
                <li class="flex items-start gap-3 px-3 py-2 text-sm">
                  <span
                    class={cn(
                      'mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-md bg-muted/60',
                      step.kind === 'tool' &&
                        step.tone === 'success' &&
                        'text-emerald-600',
                    )}
                  >
                    <Icon class="size-3.5" aria-hidden="true" />
                  </span>
                  <div class="min-w-0 flex-1 space-y-1">
                    {#if step.kind === 'edit'}
                      {@const counts = diffCounts(step.path)}
                      {@const name = step.path.split('/').pop() ?? step.path}
                      <p class="font-medium">
                        {step.verb}
                        <button
                          type="button"
                          class="font-mono text-primary underline-offset-2 hover:underline"
                          aria-label="View diff for {step.path}"
                          onclick={() => openDiffForPath(step.path)}
                        >
                          {name}
                        </button>
                        <span
                          class="ml-1 font-mono text-xs text-muted-foreground"
                          >+{counts.added} −{counts.deleted}</span
                        >
                      </p>
                      <p
                        class="truncate font-mono text-xs text-muted-foreground"
                      >
                        {step.path}
                      </p>
                    {:else}
                      <p class="font-medium">{step.title}</p>
                      {#if step.detail}
                        <p class="truncate text-xs text-muted-foreground">
                          {step.detail}
                        </p>
                      {/if}
                      {#if step.chips?.length}
                        <div class="flex flex-wrap gap-1">
                          {#each step.chips as chip (chip.label)}
                            <Badge variant="outline" class="text-[10px]"
                              >{chip.label}</Badge
                            >
                          {/each}
                        </div>
                      {/if}
                      {#if step.rawDetail}
                        <details class="text-xs text-muted-foreground">
                          <summary class="cursor-pointer select-none"
                            >Details</summary
                          >
                          <pre
                            class="mt-1 max-h-40 overflow-auto whitespace-pre-wrap rounded bg-muted/40 p-2 font-mono text-[11px]">{step.rawDetail}</pre>
                        </details>
                      {/if}
                    {/if}
                  </div>
                  <span
                    class="shrink-0 text-xs text-muted-foreground"
                    title={timeTitle(step.seq)}>{timeLabel(step.seq)}</span
                  >
                </li>
              {/each}
            </ul>
          </Card.Content>
        </Card.Root>
      {:else if item.kind === 'plan'}
        <Card.Root>
          <Card.Header class="flex-row items-center gap-2 space-y-0 pb-2">
            <List class="size-4 text-muted-foreground" aria-hidden="true" />
            <Card.Title class="text-sm font-medium">Proposed plan</Card.Title>
            <span
              class="ml-auto text-xs text-muted-foreground"
              title={timeTitle(item.seq)}>{timeLabel(item.seq)}</span
            >
          </Card.Header>
          <Card.Content>
            <ol class="list-decimal space-y-1 pl-4 text-sm">
              {#each item.steps as step, index (index)}
                <li>{step}</li>
              {/each}
            </ol>
          </Card.Content>
        </Card.Root>
      {:else if item.kind === 'approval'}
        <ApprovalCard
          {item}
          {nowMs}
          timeLabel={timeLabel(item.seq)}
          timeTitle={timeTitle(item.seq)}
          {onFocusComposer}
        />
      {:else if item.kind === 'findings'}
        <Card.Root data-od-id="findings-card">
          <Card.Header class="flex-row items-center gap-2 space-y-0 pb-2">
            <List class="size-4" aria-hidden="true" />
            <Card.Title class="text-sm font-medium">Review findings</Card.Title>
          </Card.Header>
          <Card.Content
            class="flex flex-wrap items-center justify-between gap-3"
          >
            <p class="text-sm text-muted-foreground">
              {findings.length ? findingSummary(findings) : 'No findings'}
            </p>
            <Button
              variant="secondary"
              size="sm"
              data-od-id="findings-card-open"
              onclick={onOpenFindings}
            >
              Open findings
              <ArrowRight class="size-3.5" aria-hidden="true" />
            </Button>
          </Card.Content>
        </Card.Root>
      {:else if item.kind === 'live'}
        <div
          class={cn(
            'flex items-start gap-3 rounded-lg border border-border/70 bg-muted/20 px-3 py-2',
            paused && 'opacity-90',
          )}
        >
          <span class="mt-0.5 flex size-6 items-center justify-center">
            {#if paused}
              <Pause class="size-4 text-muted-foreground" aria-hidden="true" />
            {:else}
              <LoaderCircle
                class="size-4 animate-spin text-muted-foreground"
                aria-hidden="true"
              />
            {/if}
          </span>
          <div class="min-w-0 space-y-0.5">
            <p class="text-sm font-medium">{liveTitle}</p>
            <p class="text-xs text-muted-foreground">{liveSubtitle}</p>
          </div>
        </div>
      {/if}
    </li>
  {/if}
{/each}
