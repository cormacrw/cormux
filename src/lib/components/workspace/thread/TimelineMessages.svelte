<script lang="ts">
  import { replyParts } from '$lib/thread/reply-parts'
  import { SvelteSet } from 'svelte/reactivity'
  import TaskDraftCards from './TaskDraftCards.svelte'
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
  import { workspaceRecords } from '$lib/state/workspace-records.svelte'
  import { popIn, type EntryScope } from '$lib/thread/entering'
  import {
    COLLAPSE_RUN_AT,
    editedPaths,
    toolRunSummary,
  } from '$lib/thread/tool-run-summary'
  import { cn } from '$lib/utils'
  import AgentSpinner from './AgentSpinner.svelte'
  import RunSummaryCard from './RunSummaryCard.svelte'
  import StreamingText from './StreamingText.svelte'
  import { openDiffForPath } from './open-changes'
  import ApprovalCard from './ApprovalCard.svelte'
  import QuestionCard from './QuestionCard.svelte'
  import ArrowRight from '@lucide/svelte/icons/arrow-right'
  import Check from '@lucide/svelte/icons/check'
  import Clock from '@lucide/svelte/icons/clock'
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
    activeThoughtId = null,
    activeRunId = null,
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
    /** The newest thought while it's still being written. */
    activeThoughtId?: string | null
    /** The run the agent is still in stays open; it folds once the agent moves on. */
    activeRunId?: string | null
    onOpenFindings: () => void
    onFocusComposer?: () => void
  } = $props()

  const diffFiles = $derived(workspaceDiff.filesByWorkspace[workspaceId] ?? [])
  // Media an agent references in a reply resolves against its worktree.
  const workdir = $derived(
    workspaceRecords.getRecord(workspaceId)?.worktreePath,
  )

  // What's folded isn't rendered: a thought re-renders its markdown on every chunk otherwise.
  const openFolds = new SvelteSet<string>()

  function foldToggled(id: string, event: Event) {
    if ((event.currentTarget as HTMLDetailsElement).open) openFolds.add(id)
    else openFolds.delete(id)
  }

  function planDone(statuses: string[]) {
    return statuses.filter((status) => status === 'completed').length
  }

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
      clock: Clock,
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

{#snippet stepExtras(step: ToolRunStep)}
  {#if step.kind === 'tool' && step.run && step.output}
    <li class="ml-[1.125rem] max-w-[42rem] text-sm">
      <RunSummaryCard summary={step.run} output={step.output} />
    </li>
  {/if}
  {#if step.kind === 'tool' && step.children?.length}
    <li
      class="ml-1.5 border-l-2 border-border pl-3"
      data-od-id="subagent-steps"
    >
      <ul class="space-y-1">
        {#each step.children as child (child.id)}
          {@const Icon = stepIcon(child)}
          <li
            class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
          >
            <span
              class={cn(
                'size-1.5 shrink-0 rounded-full',
                child.status === 'completed'
                  ? 'bg-success'
                  : child.status === 'failed'
                    ? 'bg-destructive'
                    : 'animate-pulse bg-primary',
              )}
              role="img"
              aria-label={child.status ?? 'running'}
            ></span>
            <Icon class="size-3 shrink-0" aria-hidden="true" />
            <span class="truncate"
              >{child.kind === 'tool'
                ? [child.title, child.detail].filter(Boolean).join(' ')
                : `${child.verb} ${child.path}`}</span
            >
          </li>
        {/each}
      </ul>
    </li>
  {/if}
{/snippet}

{#snippet runSteps(steps: ToolRunStep[])}
  <div class="space-y-1.5">
    {#each segmentSteps(steps) as segment (segment.id)}
      {#if segment.quiet}
        <ul class="space-y-1.5">
          {#each segment.steps as step, index (step.id)}
            {@const Icon = stepIcon(step)}
            {@const chips = newChips(step, segment.steps[index - 1])}
            <li
              class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
              title={timeTitle(step.atMs)}
              use:popIn={{ entries, key: step.id }}
            >
              <Icon
                class={cn(
                  'size-3 shrink-0',
                  step.kind === 'edit'
                    ? 'text-primary'
                    : 'text-muted-foreground/80',
                )}
                aria-hidden="true"
              />
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
                {#if step.children?.length}
                  <span class="shrink-0 text-muted-foreground/70"
                    >· {step.children.length} steps</span
                  >
                {/if}
              {:else if step.path}
                {@const file = diffFiles.find((row) => row.path === step.path)}
                <span class="shrink-0">{step.verb}</span>
                <button
                  type="button"
                  class="truncate font-mono text-primary underline-offset-2 hover:underline"
                  title={step.path}
                  aria-label="View diff for {step.path}"
                  onclick={() => openDiffForPath(step.path)}
                >
                  {step.path.split('/').pop()}
                </button>
                {#if file}
                  <span class="shrink-0 font-mono"
                    ><span class="text-success">+{file.added}</span>
                    <span class="text-destructive">−{file.deleted}</span></span
                  >
                {/if}
              {:else}
                <span>{step.verb} a file</span>
              {/if}
            </li>
            {@render stepExtras(step)}
          {/each}
        </ul>
      {:else}
        <!-- Notable steps: still a quiet line, set apart by weight and a tinted icon. -->
        <ul class="space-y-1.5">
          {#each segment.steps as step (step.id)}
            {@const Icon = stepIcon(step)}
            <li
              class="tool-step flex min-w-0 items-start gap-2 text-[0.8rem] text-muted-foreground"
              title={timeTitle(step.atMs)}
              use:popIn={{ entries, key: step.id }}
            >
              <Icon
                class={cn(
                  'mt-[3px] size-3.5 shrink-0',
                  step.kind === 'tool' && step.tone === 'success'
                    ? 'text-success'
                    : 'text-primary',
                )}
                aria-hidden="true"
              />
              <div class="min-w-0 flex-1">
                {#if step.kind === 'tool'}
                  <p class="flex min-w-0 flex-wrap items-baseline gap-x-1.5">
                    <span class="font-medium text-foreground/90"
                      >{step.title}</span
                    >
                    {#if step.detail}
                      <span
                        class="min-w-0 truncate font-mono text-[11px] text-muted-foreground/80"
                        >{step.detail}</span
                      >
                    {/if}
                    {#each step.chips ?? [] as chip (chip.label)}
                      <span class="text-[11px] text-muted-foreground/75"
                        >· {chip.label}</span
                      >
                    {/each}
                  </p>
                  {#if step.rawDetail}
                    <details class="mt-0.5 text-xs text-muted-foreground">
                      <summary
                        class="cursor-pointer select-none hover:text-foreground"
                        >Details</summary
                      >
                      <pre
                        class="mt-1 max-h-40 overflow-auto rounded-lg bg-muted p-2 font-mono text-(length:--code-font-size) whitespace-pre-wrap">{step.rawDetail}</pre>
                    </details>
                  {/if}
                {/if}
              </div>
              {#if timeLabel(step.atMs)}
                <span class="shrink-0 text-[11px] text-muted-foreground/70"
                  >{timeLabel(step.atMs)}</span
                >
              {/if}
            </li>
            {@render stepExtras(step)}
          {/each}
        </ul>
      {/if}
    {/each}
  </div>
{/snippet}

{#each rows as row (row.kind === 'speaker' ? row.id : row.item.id)}
  {#if row.kind === 'speaker'}
    <li
      class="flex items-center gap-2 py-1 text-xs text-muted-foreground"
      aria-hidden="true"
      data-row-key={row.id}
      use:popIn={{ entries, key: row.id }}
    >
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
      data-row-key={item.id}
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
            class="rounded-[18px] rounded-br-md user-bubble border border-[color-mix(in_oklch,var(--primary)_60%,white_10%)] px-3.5 py-2 text-sm leading-5"
          >
            <p class="whitespace-pre-wrap">{item.text}</p>
          </div>
          <button
            type="button"
            class="absolute top-1 right-1 inline-flex size-5 items-center justify-center rounded text-primary-foreground/70 opacity-0 transition hover:text-primary-foreground focus-visible:opacity-100 group-hover:opacity-100"
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
        <details
          class="group/thought max-w-[42rem]"
          data-lazy-fold
          ontoggle={(event) => foldToggled(item.id, event)}
        >
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
          {#if openFolds.has(item.id)}
            <div
              class="mt-1 border-l-2 border-border pl-3 text-sm text-muted-foreground"
            >
              <ThoughtMarkdown text={item.text} media {workdir} />
            </div>
          {/if}
        </details>
      {:else if item.kind === 'thought' && replyParts(item.text)}
        {@const taskSplit = replyParts(item.text)!}
        {@const replyText = taskSplit.text}
        <div class="group max-w-[42rem]">
          {#if replyText}
            <div class="text-sm text-foreground">
              <StreamingText
                text={replyText}
                {entries}
                key={item.id}
                live={item.id === liveReplyId}
                {workdir}
              />
            </div>
          {/if}
          {#if taskSplit.drafts.length}
            <TaskDraftCards drafts={taskSplit.drafts} />
          {/if}
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
        {#if item.steps.length >= COLLAPSE_RUN_AT && item.id !== activeRunId}
          {@const edited = editedPaths(item.steps)}
          <details
            class="group/run"
            data-od-id="tool-run-summary"
            data-lazy-fold
            ontoggle={(event) => foldToggled(item.id, event)}
          >
            <summary
              class="flex cursor-pointer items-center gap-1.5 text-xs text-muted-foreground select-none hover:text-foreground"
            >
              <ChevronRight
                class="size-3 shrink-0 transition-transform group-open/run:rotate-90"
                aria-hidden="true"
              />
              <span class="truncate">{toolRunSummary(item.steps)}</span>
            </summary>
            {#if openFolds.has(item.id)}
              <div class="mt-1.5 border-l-2 border-border pl-3">
                {@render runSteps(item.steps)}
              </div>
            {/if}
          </details>
          {#if edited.length}
            <ul class="mt-1.5 flex flex-wrap gap-1" data-od-id="files-touched">
              {#each edited as path (path)}
                {@const file = diffFiles.find((row) => row.path === path)}
                <li>
                  <button
                    type="button"
                    class="file-ref"
                    title={path}
                    aria-label="View diff for {path}"
                    onclick={() => openDiffForPath(path)}
                  >
                    {path.split('/').pop()}
                    {#if file}
                      <span class="text-success">+{file.added}</span>
                      <span class="text-destructive">−{file.deleted}</span>
                    {/if}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        {:else}
          {@render runSteps(item.steps)}
        {/if}
      {:else if item.kind === 'plan'}
        <Card.Root class="bg-popover">
          <Card.Header class="flex-row items-center gap-2 space-y-0 pb-2">
            <List class="size-4 text-muted-foreground" aria-hidden="true" />
            <Card.Title class="text-sm font-medium"
              >{item.statuses.some((status) => status !== 'pending')
                ? `Plan · ${planDone(item.statuses)} of ${item.steps.length}`
                : 'Proposed plan'}</Card.Title
            >
            {#if timeLabel(item.atMs)}
              <span
                class="ml-auto text-xs text-muted-foreground"
                title={timeTitle(item.atMs)}>{timeLabel(item.atMs)}</span
              >
            {/if}
          </Card.Header>
          <Card.Content>
            <ol class="space-y-1 text-sm" data-od-id="plan-steps">
              {#each item.steps as step, index (index)}
                {@const status = item.statuses[index]}
                <li
                  class={cn(
                    'flex items-start gap-2',
                    status === 'completed' && 'text-muted-foreground',
                  )}
                  data-status={status}
                >
                  {#if status === 'completed'}
                    <Check
                      class="mt-1 size-3.5 shrink-0 text-success"
                      aria-label="Done"
                    />
                  {:else if status === 'in_progress'}
                    <span
                      class="mt-1.5 ml-1 size-2 shrink-0 animate-pulse rounded-full bg-primary"
                      role="img"
                      aria-label="In progress"
                    ></span>
                  {:else}
                    <span
                      class="w-3.5 shrink-0 text-center text-xs leading-5 text-muted-foreground"
                      >{index + 1}</span
                    >
                  {/if}
                  <span>{step}</span>
                </li>
              {/each}
            </ol>
          </Card.Content>
        </Card.Root>
      {:else if item.kind === 'approval' && item.questions.length > 0}
        <QuestionCard
          {item}
          {nowMs}
          timeLabel={timeLabel(item.atMs)}
          timeTitle={timeTitle(item.atMs)}
          {onFocusComposer}
        />
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
