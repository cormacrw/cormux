<script lang="ts">
  import { clayColor } from '$lib/clay/identity'
  import Buddy from '$lib/components/clay/Buddy.svelte'
  import { hideFindingsBlock } from '$lib/findings/findings-block'
  import { Badge } from '$lib/components/ui/badge'
  import * as Card from '$lib/components/ui/card'
  import type { FindingRow } from '$lib/ipc/bindings'
  import ThoughtMarkdown from './ThoughtMarkdown.svelte'
  import {
    formatThreadTime,
    formatThreadTimeTitle,
  } from '$lib/thread/thread-time'
  import type {
    TimelineItem,
    TimelineRow,
    ToolRunStep,
    ToolStepIcon,
  } from '$lib/thread/timeline-types'
  import { workspaceDiff } from '$lib/state/workspace-diff.svelte'
  import { popIn, type EntryScope } from '$lib/thread/entering'
  import { cn } from '$lib/utils'
  import AgentSpinner from './AgentSpinner.svelte'
  import StreamingText from './StreamingText.svelte'
  import { openDiffForPath } from './open-changes'
  import ApprovalCard from './ApprovalCard.svelte'
  import ArrowRight from '@lucide/svelte/icons/arrow-right'
  import Check from '@lucide/svelte/icons/check'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import Copy from '@lucide/svelte/icons/copy'
  import FileText from '@lucide/svelte/icons/file-text'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import Globe from '@lucide/svelte/icons/globe'
  import List from '@lucide/svelte/icons/list'
  import Pause from '@lucide/svelte/icons/pause'
  import Play from '@lucide/svelte/icons/play'
  import Pencil from '@lucide/svelte/icons/pencil'
  import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus'
  import Search from '@lucide/svelte/icons/search'
  import Sparkles from '@lucide/svelte/icons/sparkles'
  import Square from '@lucide/svelte/icons/square'
  import Terminal from '@lucide/svelte/icons/terminal'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  let {
    rows,
    engine,
    workspaceId,
    findings,
    nowMs,
    liveTitle,
    liveSubtitle,
    paused,
    entries,
    liveReplyId = null,
    onOpenFindings,
    onFocusComposer,
  }: {
    rows: TimelineRow[]
    engine: string
    workspaceId: string
    findings: FindingRow[]
    nowMs: number
    liveTitle: string
    liveSubtitle: string
    paused: boolean
    /** Rows new to this thread pop in and replies type out; omit to render without motion. */
    entries?: EntryScope
    /** The reply the agent is still writing keeps its typing cursor. */
    liveReplyId?: string | null
    onOpenFindings: () => void
    onFocusComposer?: () => void
  } = $props()

  const diffFiles = $derived(workspaceDiff.filesByWorkspace[workspaceId] ?? [])

  // The newest thought is still being written while the live row trails it.
  const activeThoughtId = $derived.by(() => {
    const items = rows.flatMap((row) => (row.kind === 'item' ? [row.item] : []))
    if (items.at(-1)?.kind !== 'live') return null
    const last = items.at(-2)
    return last?.kind === 'thought' && last.role === 'thought' ? last.id : null
  })

  type StepSegment = { id: string; quiet: boolean; steps: ToolRunStep[] }

  // Reads, searches, commands and edits are subtle lines; anything else stays in a card.
  function segmentSteps(steps: ToolRunStep[]): StepSegment[] {
    const segments: StepSegment[] = []
    for (const step of steps) {
      const quiet = step.kind === 'edit' || Boolean(step.quiet)
      const last = segments.at(-1)
      if (last?.quiet === quiet) last.steps.push(step)
      else segments.push({ id: step.id, quiet, steps: [step] })
    }
    return segments
  }

  // A run of reads shares its chips, so only show the ones that change.
  function newChips(step: ToolRunStep, prev: ToolRunStep | undefined) {
    if (step.kind !== 'tool') return []
    const prevLabels =
      prev?.kind === 'tool' ? (prev.chips ?? []).map((chip) => chip.label) : []
    return (step.chips ?? []).filter((chip) => !prevLabels.includes(chip.label))
  }

  // A tool run's steps pop one by one, a reply types in, and the live row has its own motion.
  function popsAsRow(kind: TimelineItem['kind']) {
    return kind !== 'toolRun' && kind !== 'thought' && kind !== 'live'
  }

  function timeLabel(atMs: number) {
    return formatThreadTime(atMs) ?? ''
  }

  function timeTitle(atMs: number) {
    if (!atMs) return ''
    return formatThreadTimeTitle(atMs)
  }

  function beadClass(icon: ToolStepIcon) {
    switch (icon) {
      case 'search':
      case 'skill':
        return 'bg-plum'
      case 'pencil':
      case 'file':
        return 'bg-leaf'
      case 'terminal':
      case 'tool':
        return 'bg-leaf'
      case 'trash':
        return 'bg-brick'
      case 'globe':
        return 'bg-pond'
      default:
        return 'bg-pond'
    }
  }

  function stepIcon(step: ToolRunStep) {
    if (step.kind === 'edit') {
      return step.verb === 'Deleted' ? Trash2 : Pencil
    }
    return iconFor(step.icon)
  }

  function iconFor(icon: ToolStepIcon) {
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
      stop: Square,
      session: MessageSquarePlus,
      globe: Globe,
      skill: Sparkles,
    } as const
    return map[icon] ?? Terminal
  }

  // Titles wrap branch names in backticks; odd parts render as code.
  function titleParts(title: string) {
    return title
      .split('`')
      .map((text, index) => ({ text, code: index % 2 === 1 }))
  }

  function eventTitle(detail: string | undefined, atMs: number) {
    return [detail, timeTitle(atMs)].filter(Boolean).join('\n')
  }

  // The copy button for this item shows a check until the timer clears it.
  let copiedId = $state<string | null>(null)
  let copiedTimer: ReturnType<typeof setTimeout> | undefined

  async function copyText(id: string, text: string) {
    try {
      await navigator.clipboard.writeText(text)
    } catch {
      return
    }
    copiedId = id
    clearTimeout(copiedTimer)
    copiedTimer = setTimeout(() => (copiedId = null), 1500)
  }

  function findingSummary(items: FindingRow[]) {
    const count = (severity: string) =>
      items.filter((row) => row.severity === severity).length
    const parts = [
      [count('blocking'), 'blocking', 'blocking'],
      [count('suggestion'), 'suggestion', 'suggestions'],
      [count('nit'), 'nit', 'nits'],
    ] as const
    return parts
      .filter(([n]) => n > 0)
      .map(([n, one, many]) => `${n} ${n === 1 ? one : many}`)
      .join(', ')
  }
</script>

{#each rows as row (row.kind === 'speaker' ? row.id : row.item.id)}
  {#if row.kind === 'speaker'}
    <li
      class="flex items-center gap-2 py-1 text-xs text-muted-foreground"
      aria-hidden="true"
      use:popIn={{ entries, key: row.id }}
    >
      <Buddy color={clayColor(workspaceId)} face="happy" size={22} />
      {#if row.role}
        <span class="font-medium text-foreground">{row.role}</span>
        <span>{engine}</span>
      {:else}
        <span class="font-medium text-foreground">{engine}</span>
      {/if}
      {#if timeLabel(row.atMs)}
        <span class="text-muted-foreground/80" title={timeTitle(row.atMs)}
          >· {timeLabel(row.atMs)}</span
        >
      {/if}
    </li>
  {:else}
    {@const item = row.item}
    <li
      class={cn(
        'py-1',
        item.kind === 'user' && 'flex flex-col items-end gap-1',
      )}
      use:popIn={{
        entries,
        key: popsAsRow(item.kind) ? item.id : undefined,
        origin: item.kind === 'user' ? 'right bottom' : undefined,
      }}
    >
      {#if item.kind === 'user'}
        <div class="group relative max-w-[min(100%,34rem)]">
          <div
            class="rounded-[22px_18px_8px_20px] bg-card px-4 py-2.5 text-[15px] leading-6 text-foreground"
          >
            <p class="whitespace-pre-wrap">{item.text}</p>
          </div>
          <button
            type="button"
            class="absolute top-1 right-1 inline-flex size-5 items-center justify-center rounded text-foreground/60 opacity-0 transition hover:text-foreground focus-visible:opacity-100 group-hover:opacity-100"
            aria-label="Copy message"
            onclick={() => copyText(item.id, item.text)}
          >
            {#if copiedId === item.id}
              <Check class="size-3" aria-hidden="true" />
            {:else}
              <Copy class="size-3" aria-hidden="true" />
            {/if}
          </button>
        </div>
        {@const clock = timeLabel(item.atMs)}
        <p
          class="text-[11px] text-muted-foreground"
          title={timeTitle(item.atMs)}
        >
          You{clock ? ` · ${clock}` : ''}
        </p>
      {:else if item.kind === 'thought' && item.role === 'thought'}
        <details class="group/thought max-w-[42rem]">
          <summary
            class="inline-flex cursor-pointer list-none items-center gap-1 text-xs text-muted-foreground select-none hover:text-foreground [&::-webkit-details-marker]:hidden"
          >
            <ChevronRight
              class="size-3 transition-transform group-open/thought:rotate-90"
              aria-hidden="true"
            />
            {#if item.id === activeThoughtId}
              <span class="animate-pulse">Thinking…</span>
            {:else}
              Thought
            {/if}
          </summary>
          <div
            class="mt-1 border-l-2 border-border pl-3 text-sm text-muted-foreground"
          >
            <ThoughtMarkdown text={item.text} />
          </div>
        </details>
      {:else if item.kind === 'thought' && hideFindingsBlock(item.text)}
        {@const replyText = hideFindingsBlock(item.text)}
        <div class="group max-w-[42rem]">
          <div class="text-sm text-foreground">
            <StreamingText
              text={replyText}
              {entries}
              key={item.id}
              live={item.id === liveReplyId}
            />
          </div>
          <!-- Under the reply, not over it, so it never covers the last words of a line. -->
          <div class="mt-1 flex h-5 items-center">
            <button
              type="button"
              class="inline-flex size-5 items-center justify-center rounded text-muted-foreground opacity-0 transition hover:text-foreground focus-visible:opacity-100 group-hover:opacity-100"
              aria-label="Copy message"
              title="Copy message"
              onclick={() => copyText(item.id, replyText)}
            >
              {#if copiedId === item.id}
                <Check class="size-3" aria-hidden="true" />
              {:else}
                <Copy class="size-3" aria-hidden="true" />
              {/if}
            </button>
          </div>
        </div>
      {:else if item.kind === 'toolRun'}
        <div class="space-y-1.5">
          {#each segmentSteps(item.steps) as segment (segment.id)}
            {#if segment.quiet}
              <ul class="space-y-1.5">
                {#each segment.steps as step, index (step.id)}
                  {@const chips = newChips(step, segment.steps[index - 1])}
                  <li
                    class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
                    title={timeTitle(step.atMs)}
                    use:popIn={{ entries, key: step.id }}
                  >
                    <span
                      class="bead size-2.5 shrink-0 {step.kind === 'tool'
                        ? beadClass(step.icon)
                        : 'bg-leaf'}"
                      aria-hidden="true"
                    ></span>
                    {#if step.kind === 'tool'}
                      <span class="shrink-0">{step.title}</span>
                      {#if step.detail && step.detail !== step.title}
                        <span
                          class="truncate font-mono text-muted-foreground/80"
                          title={step.rawDetail}>{step.detail}</span
                        >
                      {/if}
                      {#each chips as chip (chip.label)}
                        <span class="shrink-0 text-muted-foreground/70"
                          >· {chip.label}</span
                        >
                      {/each}
                    {:else if step.path}
                      {@const file = diffFiles.find(
                        (row) => row.path === step.path,
                      )}
                      <span class="shrink-0">{step.verb}</span>
                      <button
                        type="button"
                        class="truncate font-mono text-foreground/80 underline-offset-2 hover:text-foreground hover:underline"
                        title={step.path}
                        aria-label="View diff for {step.path}"
                        onclick={() => openDiffForPath(step.path)}
                      >
                        {step.path.split('/').pop()}
                      </button>
                      {#if file}
                        <span class="shrink-0 font-mono"
                          ><span class="text-emerald-600">+{file.added}</span>
                          <span class="text-red-500">−{file.deleted}</span
                          ></span
                        >
                      {/if}
                    {:else}
                      <span>{step.verb} a file</span>
                    {/if}
                  </li>
                {/each}
              </ul>
            {:else}
              <Card.Root class="overflow-hidden py-0">
                <Card.Content class="p-0">
                  <ul class="divide-y divide-border/60">
                    {#each segment.steps as step (step.id)}
                      {@const Icon = stepIcon(step)}
                      <li
                        class="flex items-start gap-3 px-3 py-2 text-sm"
                        use:popIn={{ entries, key: step.id }}
                      >
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
                          {#if step.kind === 'tool'}
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
                                  class="mt-1 max-h-40 overflow-auto whitespace-pre-wrap rounded bg-muted/40 p-2 font-mono text-(length:--code-font-size)">{step.rawDetail}</pre>
                              </details>
                            {/if}
                          {/if}
                        </div>
                        {#if timeLabel(step.atMs)}
                          <span
                            class="shrink-0 text-xs text-muted-foreground"
                            title={timeTitle(step.atMs)}
                            >{timeLabel(step.atMs)}</span
                          >
                        {/if}
                      </li>
                    {/each}
                  </ul>
                </Card.Content>
              </Card.Root>
            {/if}
          {/each}
        </div>
      {:else if item.kind === 'plan'}
        <Card.Root>
          <Card.Header class="flex-row items-center gap-2 space-y-0 pb-2">
            <List class="size-4 text-muted-foreground" aria-hidden="true" />
            <Card.Title class="text-sm font-medium">Proposed plan</Card.Title>
            {#if timeLabel(item.atMs)}
              <span
                class="ml-auto text-xs text-muted-foreground"
                title={timeTitle(item.atMs)}>{timeLabel(item.atMs)}</span
              >
            {/if}
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
          timeLabel={timeLabel(item.atMs)}
          timeTitle={timeTitle(item.atMs)}
          {onFocusComposer}
        />
      {:else if item.kind === 'event'}
        {@const Icon = iconFor(item.icon)}
        <div
          class="flex items-center gap-3 py-1 text-xs text-foreground/80"
          title={eventTitle(item.detail, item.atMs)}
        >
          <span class="h-px min-w-6 flex-1 bg-border" aria-hidden="true"></span>
          <span class="flex min-w-0 items-center gap-1.5 font-medium">
            <Icon class="size-3 shrink-0" aria-hidden="true" />
            <span class="truncate"
              >{#each titleParts(item.title) as part, index (index)}{#if part.code}<span
                    class="font-mono">{part.text}</span
                  >{:else}{part.text}{/if}{/each}</span
            >
            {#if timeLabel(item.atMs)}
              <span class="shrink-0 font-normal text-muted-foreground"
                >· {timeLabel(item.atMs)}</span
              >
            {/if}
          </span>
          <span class="h-px min-w-6 flex-1 bg-border" aria-hidden="true"></span>
        </div>
      {:else if item.kind === 'findings'}
        <div
          class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
          data-od-id="findings-card"
        >
          <List class="size-3 shrink-0" aria-hidden="true" />
          <span class="shrink-0">Review findings</span>
          <span class="truncate text-muted-foreground/80"
            >· {findings.length
              ? findingSummary(findings)
              : 'No findings'}</span
          >
          <button
            type="button"
            class="inline-flex shrink-0 items-center gap-0.5 font-medium text-foreground/80 underline-offset-2 hover:text-foreground hover:underline"
            data-od-id="findings-card-open"
            onclick={onOpenFindings}
          >
            Open findings
            <ArrowRight class="size-3" aria-hidden="true" />
          </button>
        </div>
      {:else if item.kind === 'live'}
        <div class="flex min-w-0 items-center gap-2 py-0.5 text-xs">
          {#if paused}
            <Pause
              class="size-3 shrink-0 text-muted-foreground"
              aria-hidden="true"
            />
          {:else}
            <AgentSpinner class="mr-2" />
          {/if}
          <span
            class={cn(
              'shrink-0 font-medium text-foreground/80',
              // Re-added on resume as the spinner remounts, so both restart the beat together.
              !paused && 'agent-working-text',
            )}>{liveTitle}</span
          >
          {#if liveSubtitle && liveSubtitle !== liveTitle}
            <span class="truncate text-muted-foreground">· {liveSubtitle}</span>
          {/if}
        </div>
      {/if}
    </li>
  {/if}
{/each}
