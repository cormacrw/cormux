<script lang="ts">
  import { approvalResolvedChip } from '$lib/approvals/payload'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import * as Card from '$lib/components/ui/card'
  import { Textarea } from '$lib/components/ui/textarea'
  import { commands } from '$lib/ipc'
  import { coreErrorText } from '$lib/feedback/core-error'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { plans, threads, workspaceUi } from '$lib/state'
  import { cn } from '$lib/utils'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import AlertTriangle from '@lucide/svelte/icons/alert-triangle'
  import ClipboardList from '@lucide/svelte/icons/clipboard-list'
  import Check from '@lucide/svelte/icons/check'
  import X from '@lucide/svelte/icons/x'
  import ArrowRight from '@lucide/svelte/icons/arrow-right'
  import MessageSquare from '@lucide/svelte/icons/message-square'

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

  let cardEl: HTMLDivElement | undefined = $state()
  let denying = $state(false)
  let denyReason = $state('')
  let resolving = $state(false)

  const quiet = $derived(item.state !== 'pending')
  /** A plan is reviewed (and sent back with comments) in the Plan tab, not here. */
  const plan = $derived(plans.byId(item.id))
  /** A dismissed plan, or one whose tab the user closed, has nowhere to open. */
  const planOpenable = $derived.by(() => {
    if (!plan || plan.state === 'dismissed') return false
    const workspaceId = threads.getById(plan.threadId)?.workspaceId ?? ''
    return !plans.hiddenIds(workspaceId).includes(plan.id)
  })
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
          },
          item.state,
          nowMs,
        ),
  )

  async function decide(approved: boolean, reason?: string) {
    if (item.state !== 'pending' || resolving) return
    resolving = true
    try {
      const result = await commands.resolveApproval(
        item.id,
        approved,
        reason?.trim() ? reason.trim() : null,
      )
      if (result.status === 'error') {
        toastCoreError(
          coreErrorText(result.error, 'Could not resolve approval'),
        )
      } else if (result.data.focusComposer) {
        onFocusComposer?.()
      }
    } catch (error) {
      toastCoreError(error)
    } finally {
      resolving = false
      denying = false
      denyReason = ''
      queueMicrotask(() => {
        cardEl?.focus({ preventScroll: true })
      })
    }
  }

  function startDeny() {
    if (item.state !== 'pending' || resolving) return
    denying = true
  }
</script>

{#if plan}
  <!-- A plan is reviewed in the Plan tab, so the thread only needs a slim row pointing there. -->
  <div
    class="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground"
    data-od-id="approval-{item.id}"
    title={timeTitle}
  >
    {#if plan.state === 'approved'}
      <Check class="size-3 shrink-0 text-emerald-600" aria-hidden="true" />
      <span class="shrink-0">Plan approved</span>
    {:else if plan.state === 'revised'}
      <MessageSquare class="size-3 shrink-0" aria-hidden="true" />
      <span class="shrink-0">Plan sent back</span>
    {:else if plan.state === 'dismissed'}
      <X class="size-3 shrink-0" aria-hidden="true" />
      <span class="shrink-0">Left plan mode before reviewing</span>
    {:else}
      <ClipboardList class="size-3 shrink-0 text-warning" aria-hidden="true" />
      <span class="shrink-0 font-medium text-foreground/80"
        >Plan ready for review</span
      >
    {/if}
    <span class="truncate text-muted-foreground/80">· {plan.title}</span>
    {#if planOpenable}
      <button
        type="button"
        class="inline-flex shrink-0 items-center gap-0.5 font-medium text-foreground/80 underline-offset-2 hover:text-foreground hover:underline"
        data-od-id="approval-review-plan"
        onclick={() => workspaceUi.openPlan(plan.threadId)}
      >
        {plan.state === 'pending' ? 'Review plan' : 'Open plan'}
        <ArrowRight class="size-3" aria-hidden="true" />
      </button>
    {/if}
  </div>
{:else}
  <div bind:this={cardEl} tabindex="-1" class="outline-none">
    <Card.Root
      data-od-id="approval-{item.id}"
      class={cn(
        'border-warning/30 bg-warning/5',
        quiet && 'border-border/60 bg-muted/20 opacity-90',
      )}
    >
      <Card.Header class="flex-row items-start gap-2 space-y-0 pb-2">
        {#if item.state === 'approved'}
          <Check class="mt-0.5 size-4 text-emerald-600" aria-hidden="true" />
        {:else if item.state === 'denied'}
          <X class="mt-0.5 size-4 text-muted-foreground" aria-hidden="true" />
        {:else}
          <AlertTriangle
            class="mt-0.5 size-4 text-warning"
            aria-hidden="true"
          />
        {/if}
        <div class="min-w-0 flex-1 space-y-1">
          <div class="flex flex-wrap items-center gap-2">
            <Card.Title class="text-sm font-medium">{item.title}</Card.Title>
            {#if item.state === 'pending'}
              <Badge variant="outline" class="border-warning/40 text-warning"
                >Needs approval</Badge
              >
            {/if}
            <span
              class="ml-auto text-xs text-muted-foreground"
              title={timeTitle}>{timeLabel}</span
            >
          </div>
          <p class="font-mono text-sm">{item.what}</p>
          <p class="text-xs text-muted-foreground">{item.why}</p>
        </div>
      </Card.Header>
      <Card.Content class="space-y-2 pt-0">
        {#if item.state === 'pending'}
          {#if denying}
            <Textarea
              rows={2}
              placeholder="Tell the agent why (optional)"
              bind:value={denyReason}
              class="text-sm"
            />
            <div class="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="outline"
                disabled={resolving}
                onclick={() => decide(false, denyReason)}
              >
                {item.noLabel}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={resolving}
                onclick={() => {
                  denying = false
                  denyReason = ''
                }}
              >
                Cancel
              </Button>
            </div>
          {:else}
            <div class="flex flex-wrap gap-2">
              <Button
                size="sm"
                class="rounded-full bg-emerald-600 text-white hover:bg-emerald-600/90"
                disabled={resolving}
                onclick={() => decide(true)}
              >
                <Check class="size-3.5" aria-hidden="true" />
                {item.okLabel}
              </Button>
              <Button
                size="sm"
                variant="outline"
                class="rounded-full"
                disabled={resolving}
                onclick={startDeny}
              >
                {item.noLabel}
              </Button>
            </div>
          {/if}
        {:else if resolvedChip}
          <Badge
            variant="outline"
            class={cn(
              item.state === 'approved' &&
                'border-emerald-500/40 text-emerald-700',
            )}
          >
            {resolvedChip}
          </Badge>
        {/if}
      </Card.Content>
    </Card.Root>
  </div>
{/if}
