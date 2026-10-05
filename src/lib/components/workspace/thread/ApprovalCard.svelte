<script lang="ts">
  import { approvalResolvedChip } from '$lib/approvals/payload'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import * as Card from '$lib/components/ui/card'
  import { Textarea } from '$lib/components/ui/textarea'
  import { commands } from '$lib/ipc'
  import { coreErrorText } from '$lib/feedback/core-error'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { cn } from '$lib/utils'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import AlertTriangle from '@lucide/svelte/icons/alert-triangle'
  import Check from '@lucide/svelte/icons/check'
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

  let cardEl: HTMLDivElement | undefined = $state()
  let denying = $state(false)
  let denyReason = $state('')
  let resolving = $state(false)

  const quiet = $derived(item.state !== 'pending')
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

<div bind:this={cardEl} tabindex="-1" class="outline-none">
  <Card.Root
    data-od-id="approval-{item.id}"
    class={cn('!bg-butter', quiet && '!bg-muted opacity-90')}
  >
    <Card.Header class="flex-row items-start gap-2 space-y-0 pb-2">
      {#if item.state === 'approved'}
        <Check class="mt-0.5 size-4 text-emerald-600" aria-hidden="true" />
      {:else if item.state === 'denied'}
        <X class="mt-0.5 size-4 text-muted-foreground" aria-hidden="true" />
      {:else}
        <AlertTriangle class="mt-0.5 size-4 text-warning" aria-hidden="true" />
      {/if}
      <div class="min-w-0 flex-1 space-y-1">
        <div class="flex flex-wrap items-center gap-2">
          <Card.Title class="font-display text-lg font-extrabold">{item.title}</Card.Title>
          {#if item.state === 'pending'}
            <Badge class="!bg-card font-display font-extrabold text-cocoa"
              >Needs approval</Badge
            >
          {/if}
          <span class="ml-auto text-xs text-muted-foreground" title={timeTitle}
            >{timeLabel}</span
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
              class="!bg-leaf !text-cocoa"
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
