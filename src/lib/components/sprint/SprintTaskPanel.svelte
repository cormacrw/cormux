<script lang="ts">
  import { onDestroy } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import ThoughtMarkdown from '$lib/components/workspace/thread/ThoughtMarkdown.svelte'
  import {
    formatPoints,
    parsePoints,
    POINT_PRESETS,
    statusLabel,
    taskRef,
  } from '$lib/clickup/board'
  import { clickup } from '$lib/state'
  import { cn } from '$lib/utils'
  import Check from '@lucide/svelte/icons/check'
  import Copy from '@lucide/svelte/icons/copy'
  import ExternalLink from '@lucide/svelte/icons/external-link'
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert'
  import X from '@lucide/svelte/icons/x'
  import { openUrl } from '@tauri-apps/plugin-opener'

  const task = $derived(clickup.selectedTask)
  const detail = $derived(task ? clickup.details[task.id] : undefined)
  const ref = $derived(task ? taskRef(task) : '')

  let copied = $state(false)
  let copiedTimer: ReturnType<typeof setTimeout> | undefined

  async function copyRef() {
    try {
      await navigator.clipboard.writeText(ref)
    } catch {
      return
    }
    copied = true
    clearTimeout(copiedTimer)
    copiedTimer = setTimeout(() => (copied = false), 1500)
  }

  onDestroy(() => clearTimeout(copiedTimer))

  let pointsDraft = $state('')
  let titleEl: HTMLHeadingElement | undefined = $state()

  $effect(() => {
    if (!task) return
    pointsDraft = task.points == null ? '' : formatPoints(task.points)
  })

  // Opening a task from Homebase or the board moves focus here for keyboard users.
  $effect(() => {
    if (task?.id) titleEl?.focus({ preventScroll: true })
  })

  function commitDraft() {
    if (!task) return
    const points = parsePoints(pointsDraft)
    if (points == null) {
      pointsDraft = task.points == null ? '' : formatPoints(task.points)
      return
    }
    void clickup.setPoints(task.id, points)
  }

  const DATE = new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  })
</script>

{#if task}
  <aside
    class="flex h-full min-h-0 flex-col bg-background"
    aria-labelledby="sprint-task-title"
    data-od-id="sprint-task-panel"
  >
    <header
      class="flex items-center gap-2 border-b border-border px-4 py-2.5 text-xs text-muted-foreground"
    >
      <button
        type="button"
        class="inline-flex items-center gap-1.5 rounded-md border border-border bg-muted/50 px-2 py-1 font-mono text-[11px] text-foreground outline-none transition-colors hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50"
        aria-label={copied ? `Copied ${ref}` : `Copy ${ref}`}
        title="Copy ID"
        data-od-id="sprint-task-copy-id"
        onclick={copyRef}
      >
        {ref}
        {#if copied}
          <Check class="size-3 text-success" aria-hidden="true" />
        {:else}
          <Copy class="size-3 text-muted-foreground" aria-hidden="true" />
        {/if}
      </button>
      {#if detail?.listName}
        <span aria-hidden="true">·</span>
        <span class="truncate">{detail.listName}</span>
      {/if}
      <div class="ml-auto flex items-center gap-1">
        <Button
          variant="ghost"
          size="sm"
          class="h-7 gap-1.5 px-2 text-xs"
          onclick={() => void openUrl(task.url)}
        >
          <ExternalLink class="size-3.5" aria-hidden="true" />
          Open in ClickUp
        </Button>
        <Button
          variant="ghost"
          size="icon"
          class="size-7"
          aria-label="Close task"
          onclick={() => clickup.select(null)}
        >
          <X class="size-4" aria-hidden="true" />
        </Button>
      </div>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto">
      <div class="flex flex-col gap-5 p-4">
        <h2
          bind:this={titleEl}
          id="sprint-task-title"
          tabindex="-1"
          class="text-lg leading-snug font-semibold tracking-tight outline-none"
        >
          {task.name}
        </h2>

        <dl
          class="grid grid-cols-[96px_minmax(0,1fr)] items-center gap-x-3 gap-y-3 text-sm"
        >
          <dt class="text-xs text-muted-foreground">Status</dt>
          <dd>
            <span
              class="inline-flex items-center gap-2 rounded-full border border-border px-2.5 py-0.5 text-xs font-medium"
              data-od-id="sprint-task-status"
            >
              <span
                class="size-2 shrink-0 rounded-full"
                style:background-color={task.statusColor ?? 'currentColor'}
                aria-hidden="true"
              ></span>
              {statusLabel(task.status)}
            </span>
          </dd>

          <dt class="self-start pt-1.5 text-xs text-muted-foreground">
            Sprint points
          </dt>
          <dd class="grid gap-2">
            {#if task.points == null}
              <p
                class="inline-flex w-fit items-center gap-1.5 rounded-md border border-warning/50 bg-warning/15 px-2 py-1 text-xs font-medium text-warning"
                data-od-id="sprint-task-unpointed"
              >
                <TriangleAlert class="size-3.5" aria-hidden="true" />
                Not estimated yet
              </p>
            {/if}
            <div
              class="flex flex-wrap items-center gap-1"
              role="group"
              aria-label="Sprint points"
            >
              {#each POINT_PRESETS as value (value)}
                <button
                  type="button"
                  class={cn(
                    'h-8 min-w-8 rounded-md border px-2 font-mono text-xs outline-none transition-colors focus-visible:ring-3 focus-visible:ring-ring/50',
                    task.points === value
                      ? 'border-primary bg-primary text-primary-foreground'
                      : 'border-border hover:bg-muted',
                  )}
                  aria-pressed={task.points === value}
                  aria-label="{value} points"
                  onclick={() => void clickup.setPoints(task.id, value)}
                >
                  {value}
                </button>
              {/each}
              <Input
                class="h-8 w-16 font-mono text-xs"
                inputmode="decimal"
                placeholder="Other"
                aria-label="Other points value"
                bind:value={pointsDraft}
                onblur={commitDraft}
                onkeydown={(event) => {
                  if (event.key === 'Enter') {
                    event.preventDefault()
                    commitDraft()
                  }
                }}
              />
            </div>
          </dd>

          {#if task.assignees.length > 0}
            <dt class="text-xs text-muted-foreground">Assignees</dt>
            <dd class="flex flex-wrap gap-1.5">
              {#each task.assignees as user (user.id)}
                <span
                  class="inline-flex items-center gap-1.5 rounded-full border border-border py-0.5 pr-2 pl-0.5 text-xs"
                >
                  <span
                    class="flex size-5 items-center justify-center rounded-full text-[9px] font-semibold text-white"
                    style:background-color={user.color ??
                      'var(--muted-foreground)'}
                    aria-hidden="true">{user.initials}</span
                  >
                  {user.username}
                </span>
              {/each}
            </dd>
          {/if}

          {#if task.priority}
            <dt class="text-xs text-muted-foreground">Priority</dt>
            <dd class="flex items-center gap-2 text-sm">
              <span
                class="size-2 rounded-full"
                style:background-color={task.priorityColor ?? 'currentColor'}
                aria-hidden="true"
              ></span>
              {statusLabel(task.priority)}
            </dd>
          {/if}

          {#if detail?.dueMs}
            <dt class="text-xs text-muted-foreground">Due</dt>
            <dd class="text-sm">{DATE.format(detail.dueMs)}</dd>
          {/if}

          {#if detail && detail.tags.length > 0}
            <dt class="text-xs text-muted-foreground">Tags</dt>
            <dd class="flex flex-wrap gap-1">
              {#each detail.tags as tag (tag.name)}
                <span
                  class="rounded px-1.5 py-0.5 text-[11px] font-medium"
                  style:background-color={tag.bg}
                  style:color={tag.fg}>{tag.name}</span
                >
              {/each}
            </dd>
          {/if}
        </dl>

        <section class="grid gap-2" aria-labelledby="sprint-task-desc">
          <h3
            id="sprint-task-desc"
            class="text-xs font-medium text-muted-foreground"
          >
            Description
          </h3>
          {#if detail}
            {#if detail.description.trim()}
              <div class="text-sm">
                <ThoughtMarkdown text={detail.description} rich />
              </div>
            {:else}
              <p class="text-sm text-muted-foreground">No description.</p>
            {/if}
          {:else if clickup.detailError}
            <p class="text-sm text-destructive" role="alert">
              {clickup.detailError}
            </p>
          {:else}
            <div class="grid gap-2" aria-label="Loading description">
              <div class="h-3 w-11/12 animate-pulse rounded bg-muted"></div>
              <div class="h-3 w-4/5 animate-pulse rounded bg-muted"></div>
              <div class="h-3 w-2/3 animate-pulse rounded bg-muted"></div>
            </div>
          {/if}
        </section>

        {#if detail?.creator || detail?.createdMs}
          <p class="text-xs text-muted-foreground">
            Created{detail.creator
              ? ` by ${detail.creator.username}`
              : ''}{detail.createdMs
              ? ` on ${DATE.format(detail.createdMs)}`
              : ''}
          </p>
        {/if}
      </div>
    </div>
  </aside>
{/if}
