<script lang="ts">
  import { tick } from 'svelte'
  import ArrowRight from '@lucide/svelte/icons/arrow-right'
  import Check from '@lucide/svelte/icons/check'
  import CommandIcon from '@lucide/svelte/icons/command'
  import CornerDownLeftIcon from '@lucide/svelte/icons/corner-down-left'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import MessageSquare from '@lucide/svelte/icons/message-square'
  import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import { Button } from '$lib/components/ui/button'
  import { Kbd } from '$lib/components/ui/kbd'
  import * as Select from '$lib/components/ui/select'
  import { Textarea } from '$lib/components/ui/textarea'
  import ThoughtMarkdown from './thread/ThoughtMarkdown.svelte'
  import { agentName } from '$lib/agent-name'
  import { showToast } from '$lib/feedback/show-toast'
  import { formatPlanFeedback } from '$lib/plans/feedback'
  import { offsetsToRange, rangeToOffsets } from '$lib/plans/text-offsets'
  import { engineDisplayName } from '$lib/sidebar/engine'
  import { plural } from '$lib/sidebar/status'
  import { app, plans, threads, workspaceUi } from '$lib/state'
  import { cn } from '$lib/utils'

  let {
    workspaceId,
    headingRef = $bindable(),
  }: {
    workspaceId: string
    headingRef?: HTMLHeadingElement | undefined
  } = $props()

  type Spot = {
    start: number
    end: number
    quote: string
    top: number
    left: number
  }

  let bodyEl: HTMLDivElement | undefined = $state()
  let contentEl: HTMLDivElement | undefined = $state()
  let markdownEl: HTMLDivElement | undefined = $state()
  /** The highlighted text, waiting for the Comment button. */
  let picked = $state<Spot | null>(null)
  /** The text a comment is being written on. */
  let composing = $state<Spot | null>(null)
  let draftBody = $state('')
  let activeCommentId = $state<string | null>(null)
  let busy = $state<'approve' | 'feedback' | null>(null)
  let error = $state<string | null>(null)

  const wsThreads = $derived(threads.forWorkspace(workspaceId))
  const latest = $derived(
    plans.visibleFor(
      workspaceId,
      wsThreads.map((thread) => thread.id),
    ),
  )
  const plan = $derived(
    latest.find((entry) => entry.threadId === workspaceUi.planThreadId) ??
      latest.find((entry) => entry.state === 'pending') ??
      latest[0],
  )
  const thread = $derived(plan ? threads.getById(plan.threadId) : undefined)
  const name = $derived(plan ? agentName(plan.threadId) : 'Agent')
  const pending = $derived(plan?.state === 'pending')
  const comments = $derived(
    plan
      ? [...plans.commentsFor(plan.id)].sort((a, b) => a.start - b.start)
      : [],
  )
  const note = $derived(plan ? (plans.notes[plan.id] ?? '') : '')
  const hasFeedback = $derived(comments.length > 0 || note.trim().length > 0)

  // A different plan (a revision, or another thread's) starts with nothing picked.
  $effect(() => {
    void plan?.id
    picked = null
    composing = null
    draftBody = ''
    error = null
    queueMicrotask(() => {
      if (bodyEl) bodyEl.scrollTop = 0
    })
  })

  function spotFor(range: Range): Spot | null {
    if (!markdownEl || !contentEl) return null
    const offsets = rangeToOffsets(markdownEl, range)
    const quote = range.toString().trim()
    if (!offsets || !quote) return null
    const rect = range.getBoundingClientRect()
    const base = contentEl.getBoundingClientRect()
    return {
      ...offsets,
      quote,
      top: rect.bottom - base.top + 6,
      left: Math.max(0, Math.min(rect.left - base.left, base.width - 352)),
    }
  }

  // Selecting text in a plan that's waiting offers a Comment button under it.
  $effect(() => {
    if (!pending) return
    const onSelectionChange = () => {
      if (composing) return
      const selection = document.getSelection()
      picked =
        selection && selection.rangeCount > 0
          ? spotFor(selection.getRangeAt(0))
          : null
    }
    document.addEventListener('selectionchange', onSelectionChange)
    return () =>
      document.removeEventListener('selectionchange', onSelectionChange)
  })

  function startComment() {
    if (!picked) return
    composing = picked
    picked = null
    draftBody = ''
    document.getSelection()?.removeAllRanges()
  }

  function saveComment() {
    if (!plan || !composing || !draftBody.trim()) return
    plans.addComment(plan.id, {
      start: composing.start,
      end: composing.end,
      quote: composing.quote,
      body: draftBody.trim(),
    })
    composing = null
    draftBody = ''
  }

  function cancelComment() {
    composing = null
    draftBody = ''
  }

  // Commented text stays highlighted, using the CSS Custom Highlight API so the
  // rendered markdown is never rewrapped. Re-run whenever the plan or comments change.
  $effect(() => {
    const root = markdownEl
    const markdown = plan?.markdown
    const spans = comments.map((comment) => ({
      id: comment.id,
      start: comment.start,
      end: comment.end,
    }))
    const draft = composing
    const active = activeCommentId
    if (!root || markdown === undefined || typeof Highlight === 'undefined')
      return
    void tick().then(() => {
      const ranges: Range[] = []
      const focused: Range[] = []
      for (const span of spans) {
        const range = offsetsToRange(root, span.start, span.end)
        if (!range) continue
        ;(span.id === active ? focused : ranges).push(range)
      }
      const draftRange = draft
        ? offsetsToRange(root, draft.start, draft.end)
        : null
      if (draftRange) focused.push(draftRange)
      CSS.highlights.set('plan-comment', new Highlight(...ranges))
      CSS.highlights.set('plan-comment-active', new Highlight(...focused))
    })
    return () => {
      CSS.highlights.delete('plan-comment')
      CSS.highlights.delete('plan-comment-active')
    }
  })

  let flashTimer: ReturnType<typeof setTimeout> | undefined
  function revealComment(id: string, start: number, end: number) {
    if (!markdownEl || !bodyEl) return
    const range = offsetsToRange(markdownEl, start, end)
    if (range) {
      const rect = range.getBoundingClientRect()
      const box = bodyEl.getBoundingClientRect()
      bodyEl.scrollTo({
        top: bodyEl.scrollTop + rect.top - box.top - box.height / 3,
        behavior: 'smooth',
      })
    }
    activeCommentId = id
    clearTimeout(flashTimer)
    flashTimer = setTimeout(() => (activeCommentId = null), 1600)
  }

  async function sendFeedback() {
    if (!plan || !hasFeedback || busy) return
    busy = 'feedback'
    error = null
    const count = comments.length
    try {
      await plans.resolve(plan.id, formatPlanFeedback(comments, note))
      showToast({
        tone: 'ok',
        parts: [
          {
            type: 'text',
            value: count
              ? `Sent ${plural(count, 'comment')} to ${name}`
              : `Sent your feedback to ${name}`,
          },
        ],
      })
    } catch (caught) {
      error = caught instanceof Error ? caught.message : 'Could not send'
    } finally {
      busy = null
    }
  }

  async function approve() {
    if (!plan || hasFeedback || busy) return
    busy = 'approve'
    error = null
    try {
      await plans.resolve(plan.id, null)
      goToThread()
    } catch (caught) {
      error = caught instanceof Error ? caught.message : 'Could not approve'
    } finally {
      busy = null
    }
  }

  function goToThread() {
    if (!plan) return
    app.threadId = plan.threadId
    workspaceUi.openTab('thread')
  }
</script>

<div
  id="plan-panel"
  role="tabpanel"
  aria-labelledby="thread-tab-plan"
  data-od-id="plan"
  class="flex min-h-0 flex-1 flex-col"
>
  {#if plan}
    <div
      bind:this={bodyEl}
      id="plan-body"
      class="min-h-0 flex-1 overflow-y-auto"
    >
      <div
        class="mx-auto grid w-full max-w-6xl gap-8 px-8 pt-8 pb-12 lg:grid-cols-[minmax(0,1fr)_18rem]"
      >
        <div class="min-w-0">
          <header class="space-y-3 border-b border-border/60 pb-5">
            <div
              class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground"
            >
              <span
                >Plan from {name} with {engineDisplayName(
                  thread?.engine ?? 'claude',
                )}{plan.version > 1 ? ` · revision ${plan.version}` : ''}</span
              >
              {#if latest.length > 1}
                <label class="ml-auto flex items-center gap-2">
                  <span>Showing</span>
                  <Select.Root
                    type="single"
                    value={plan.threadId}
                    onValueChange={(value) => {
                      if (value) workspaceUi.planThreadId = value
                    }}
                  >
                    <Select.Trigger
                      size="sm"
                      class="text-xs text-foreground"
                      data-od-id="plan-thread"
                    >
                      {name}
                    </Select.Trigger>
                    <Select.Content>
                      {#each latest as entry (entry.id)}
                        <Select.Item
                          value={entry.threadId}
                          label={agentName(entry.threadId)}
                        >
                          {agentName(entry.threadId)}
                          {#if entry.state === 'pending'}
                            <span class="ml-auto text-xs text-muted-foreground"
                              >waiting</span
                            >
                          {/if}
                        </Select.Item>
                      {/each}
                    </Select.Content>
                  </Select.Root>
                </label>
              {/if}
            </div>
            <h2
              bind:this={headingRef}
              id="plan-heading"
              tabindex="-1"
              class="flex items-center gap-2 text-xl font-semibold tracking-tight outline-none"
            >
              {#if pending}
                Review the plan
              {:else if plan.state === 'approved'}
                <Check class="size-5 text-emerald-600" aria-hidden="true" />
                Plan approved
              {:else}
                Feedback sent
              {/if}
            </h2>
            {#if pending}
              <p class="text-sm text-muted-foreground" data-od-id="plan-status">
                Nothing is edited until you approve. Highlight any part of the
                plan to comment on it, then send your comments back or approve
                it.
              </p>
            {:else if plan.state === 'approved'}
              <p class="text-sm text-muted-foreground" data-od-id="plan-status">
                {name} is carrying out this plan.
              </p>
            {:else}
              <div class="space-y-2" data-od-id="plan-status">
                <p
                  class="flex items-center gap-1.5 text-sm text-muted-foreground"
                >
                  {#if thread?.status === 'running'}
                    <LoaderCircle
                      class="size-4 animate-spin"
                      aria-hidden="true"
                    />
                  {/if}
                  {name} is revising the plan.
                </p>
                {#if plan.feedback}
                  <details class="text-sm">
                    <summary class="cursor-pointer text-muted-foreground"
                      >What you sent</summary
                    >
                    <p
                      class="mt-2 rounded-md border border-border/60 bg-muted/30 p-3 whitespace-pre-wrap"
                    >
                      {plan.feedback}
                    </p>
                  </details>
                {/if}
              </div>
            {/if}
          </header>

          <div bind:this={contentEl} class="relative mt-6">
            <div
              bind:this={markdownEl}
              class={cn('plan-markdown select-text', !pending && 'opacity-90')}
              data-od-id="plan-markdown"
            >
              <ThoughtMarkdown text={plan.markdown} rich />
            </div>

            {#if pending && picked && !composing}
              <Button
                size="sm"
                class="absolute z-10 shadow-md"
                style="top: {picked.top}px; left: {picked.left}px"
                data-od-id="plan-comment-start"
                onmousedown={(event) => event.preventDefault()}
                onclick={startComment}
              >
                <MessageSquarePlus class="size-3.5" aria-hidden="true" />
                Comment
              </Button>
            {/if}

            {#if composing}
              <form
                class="plan-comment-card absolute z-10 flex w-[22rem] flex-col gap-2 p-2.5 shadow-lg"
                style="top: {composing.top}px; left: {composing.left}px"
                data-od-id="plan-comment-form"
                onsubmit={(event) => {
                  event.preventDefault()
                  saveComment()
                }}
              >
                <p
                  class="flex items-center gap-1.5 text-xs font-medium text-muted-foreground"
                >
                  <MessageSquare
                    class="size-3.5 text-info"
                    aria-hidden="true"
                  />
                  New comment
                </p>
                <Textarea
                  bind:value={draftBody}
                  aria-label="Comment on the highlighted text"
                  placeholder="What should change here?"
                  class="min-h-16 bg-background text-sm"
                  autofocus
                  onkeydown={(event) => {
                    if (
                      event.key === 'Enter' &&
                      (event.metaKey || event.ctrlKey)
                    ) {
                      event.preventDefault()
                      saveComment()
                    } else if (event.key === 'Escape') {
                      event.stopPropagation()
                      cancelComment()
                    }
                  }}
                />
                <div class="flex justify-end gap-2">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onclick={cancelComment}
                    >Cancel<Kbd class="ml-1" aria-hidden="true">Esc</Kbd
                    ></Button
                  >
                  <Button type="submit" size="sm" disabled={!draftBody.trim()}
                    >Add comment<Kbd class="ml-1 gap-0.5" aria-hidden="true"
                      ><CommandIcon /><CornerDownLeftIcon /></Kbd
                    ></Button
                  >
                </div>
              </form>
            {/if}
          </div>
        </div>

        <aside
          class="self-start lg:sticky lg:top-0"
          aria-label="Comments on the plan"
          data-od-id="plan-comments"
        >
          <h3 class="mb-3 text-xs font-medium text-muted-foreground">
            {#if comments.length}
              {plural(comments.length, 'comment')}, not sent yet
            {:else if pending}
              No comments yet
            {:else}
              Comments
            {/if}
          </h3>
          {#if comments.length}
            <ul class="space-y-2">
              {#each comments as comment (comment.id)}
                <li
                  class={cn(
                    'group plan-comment-card flex items-start gap-2 p-2.5 transition-colors',
                    activeCommentId === comment.id && 'bg-info/10',
                  )}
                  data-od-id="plan-comment"
                >
                  <button
                    type="button"
                    class="min-w-0 flex-1 text-left"
                    title="Show in the plan"
                    onclick={() =>
                      revealComment(comment.id, comment.start, comment.end)}
                  >
                    <span
                      class="line-clamp-2 border-l-2 border-info/50 pl-2 text-xs text-muted-foreground"
                      >{comment.quote}</span
                    >
                    <span class="mt-1.5 block text-sm whitespace-pre-wrap"
                      >{comment.body}</span
                    >
                  </button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    class="text-muted-foreground opacity-0 group-focus-within:opacity-100 group-hover:opacity-100 hover:text-destructive"
                    aria-label="Delete comment"
                    title="Delete comment"
                    onclick={() => plans.removeComment(plan.id, comment.id)}
                  >
                    <Trash2 class="size-3.5" aria-hidden="true" />
                  </Button>
                </li>
              {/each}
            </ul>
          {:else if pending}
            <p class="text-sm text-muted-foreground">
              Select text in the plan, then choose Comment.
            </p>
          {/if}
        </aside>
      </div>
    </div>

    <div
      id="plan-bar"
      data-od-id="plan-bar"
      class="border-t border-border/50 bg-popover"
    >
      <div
        class="mx-auto flex w-full max-w-6xl flex-wrap items-end gap-3 px-8 py-3"
      >
        {#if pending}
          <Textarea
            rows={1}
            value={note}
            aria-label="Overall feedback on the plan"
            placeholder="Overall feedback (optional)"
            class="max-h-32 min-h-9 min-w-56 flex-1 resize-none text-sm"
            data-od-id="plan-note"
            oninput={(event) =>
              plans.setNote(plan.id, event.currentTarget.value)}
            onkeydown={(event) => {
              if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
                event.preventDefault()
                void sendFeedback()
              }
            }}
          />
          {#if error}
            <p class="self-center text-xs text-destructive">{error}</p>
          {/if}
          <Button
            variant="outline"
            data-od-id="plan-send-feedback"
            disabled={!hasFeedback || busy !== null}
            onclick={() => void sendFeedback()}
          >
            {#if busy === 'feedback'}
              <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
            {:else}
              <ArrowRight class="size-4" aria-hidden="true" />
            {/if}
            {comments.length
              ? `Send ${plural(comments.length, 'comment')}`
              : 'Send feedback'}
          </Button>
          <Button
            class="bg-emerald-600 text-white hover:bg-emerald-600/90"
            data-od-id="plan-approve"
            disabled={hasFeedback || busy !== null}
            title={hasFeedback
              ? 'Send or delete your feedback before approving'
              : `${name} starts carrying out the plan`}
            onclick={() => void approve()}
          >
            {#if busy === 'approve'}
              <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
            {:else}
              <Check class="size-4" aria-hidden="true" />
            {/if}
            Approve plan
          </Button>
        {:else}
          <p class="self-center text-sm text-muted-foreground">
            {plan.state === 'approved'
              ? `Follow the work in ${name}’s thread.`
              : `The revised plan opens here when ${name} is done.`}
          </p>
          <span class="grow"></span>
          <Button variant="secondary" onclick={goToThread}>
            Go to {name}
            <ArrowRight class="size-4" aria-hidden="true" />
          </Button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .plan-comment-card {
    border: 1px solid color-mix(in oklch, var(--info) 35%, transparent);
    border-radius: var(--radius);
    background: var(--popover);
  }
</style>
