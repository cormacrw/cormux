<script lang="ts">
  import { approvalResolvedChip, composeAnswer } from '$lib/approvals/payload'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import * as Card from '$lib/components/ui/card'
  import { Textarea } from '$lib/components/ui/textarea'
  import { commands, type CoreError } from '$lib/ipc'
  import { coreErrorText } from '$lib/feedback/core-error'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { cn } from '$lib/utils'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import Check from '@lucide/svelte/icons/check'
  import CircleHelp from '@lucide/svelte/icons/circle-help'
  import X from '@lucide/svelte/icons/x'

  type ApprovalItem = Extract<TimelineItem, { kind: 'approval' }>

  let {
    item,
    nowMs,
    timeLabel,
    timeTitle,
    onFocusComposer,
  }: {
    item: ApprovalItem
    nowMs: number
    timeLabel: string
    timeTitle: string
    onFocusComposer?: () => void
  } = $props()

  // Both keyed by question text, the way the agent wants its answers back.
  let selected = $state<Record<string, string[]>>({})
  let typed = $state<Record<string, string>>({})
  let resolving = $state(false)

  const pending = $derived(item.state === 'pending')
  const answers = $derived(
    Object.fromEntries(
      item.questions.map(({ question }) => [
        question,
        composeAnswer(selected[question] ?? [], typed[question] ?? ''),
      ]),
    ),
  )
  const ready = $derived(Object.values(answers).every(Boolean))
  const resolvedChip = $derived(
    item.state === 'pending'
      ? null
      : approvalResolvedChip(
          {
            title: item.title,
            what: item.what,
            why: item.why,
            okLabel: item.okLabel,
            noLabel: item.noLabel,
            resolvedAtMs: item.doneAtMs ?? undefined,
            questions: item.questions,
          },
          item.state,
          nowMs,
        ),
  )

  function toggle(question: string, label: string, multiSelect: boolean) {
    const current = selected[question] ?? []
    if (current.includes(label)) {
      selected[question] = current.filter((entry) => entry !== label)
    } else {
      selected[question] = multiSelect ? [...current, label] : [label]
    }
  }

  async function finish(
    send: () => Promise<
      { status: 'ok' } | { status: 'error'; error: CoreError }
    >,
  ) {
    if (!pending || resolving) return
    resolving = true
    try {
      const result = await send()
      if (result.status === 'error') {
        toastCoreError(coreErrorText(result.error, 'Could not send the answer'))
      } else {
        onFocusComposer?.()
      }
    } catch (error) {
      toastCoreError(error)
    } finally {
      resolving = false
    }
  }

  function submit() {
    if (ready) void finish(() => commands.answerQuestion(item.id, answers))
  }

  function skip() {
    void finish(() => commands.resolveApproval(item.id, false, null))
  }

  function onTextKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
      event.preventDefault()
      submit()
    }
  }
</script>

<Card.Root
  data-od-id="question-{item.id}"
  class={cn(
    'border-warning/40 bg-popover',
    !pending && 'border-border/60 opacity-90',
  )}
>
  <Card.Header class="pb-2">
    <div class="flex flex-wrap items-center gap-2">
      {#if item.state === 'approved'}
        <Check class="size-4 text-emerald-600" aria-hidden="true" />
      {:else if item.state === 'denied'}
        <X class="size-4 text-muted-foreground" aria-hidden="true" />
      {:else}
        <CircleHelp class="size-4 text-warning" aria-hidden="true" />
      {/if}
      <Card.Title class="text-sm font-medium">{item.title}</Card.Title>
      <span class="ml-auto text-xs text-muted-foreground" title={timeTitle}
        >{timeLabel}</span
      >
    </div>
  </Card.Header>
  <Card.Content class="space-y-4 pt-0">
    {#each item.questions as { question, header, options, multiSelect } (question)}
      <div class="space-y-2">
        {#if header}
          <p class="text-xs font-medium text-muted-foreground">{header}</p>
        {/if}
        <p class="text-sm">{question}</p>
        {#if pending}
          <div
            class="grid gap-2 sm:grid-cols-2"
            role="group"
            aria-label={question}
          >
            {#each options as { label, description } (label)}
              {@const picked = selected[question]?.includes(label) ?? false}
              <button
                type="button"
                aria-pressed={picked}
                disabled={resolving}
                class={cn(
                  'rounded-md border px-3 py-2 text-left text-sm transition-colors hover:bg-accent disabled:opacity-60',
                  picked && 'border-emerald-500/60 bg-emerald-500/10',
                )}
                onclick={() => toggle(question, label, multiSelect)}
              >
                <span class="block font-medium">{label}</span>
                {#if description}
                  <span class="block text-xs text-muted-foreground"
                    >{description}</span
                  >
                {/if}
              </button>
            {/each}
          </div>
          <Textarea
            rows={1}
            placeholder={options.length
              ? 'Add your own answer or anything else the agent should know'
              : 'Type your answer'}
            aria-label="Your answer to: {question}"
            disabled={resolving}
            value={typed[question] ?? ''}
            oninput={(event) => (typed[question] = event.currentTarget.value)}
            onkeydown={onTextKeydown}
            class="text-sm"
          />
        {:else if item.answers[question]}
          <p class="text-sm font-medium">{item.answers[question]}</p>
        {/if}
      </div>
    {/each}
    {#if pending}
      <div class="flex flex-wrap gap-2">
        <Button
          size="sm"
          class="rounded-full bg-emerald-600 text-white hover:bg-emerald-600/90"
          disabled={!ready || resolving}
          onclick={submit}
        >
          <Check class="size-3.5" aria-hidden="true" />
          {item.okLabel}
        </Button>
        <Button
          size="sm"
          variant="outline"
          class="rounded-full"
          disabled={resolving}
          onclick={skip}
        >
          {item.noLabel}
        </Button>
      </div>
    {:else if resolvedChip}
      <Badge
        variant="outline"
        class={cn(
          item.state === 'approved' && 'border-emerald-500/40 text-emerald-700',
        )}
      >
        {resolvedChip}
      </Badge>
    {/if}
  </Card.Content>
</Card.Root>
