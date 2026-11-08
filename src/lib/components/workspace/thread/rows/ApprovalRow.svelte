<!-- A permission the agent asked for, or a question it put to you: each has its own card. -->
<script lang="ts">
  import {
    formatThreadTime,
    formatThreadTimeTitle,
  } from '$lib/thread/thread-time'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import ApprovalCard from '../ApprovalCard.svelte'
  import QuestionCard from '../QuestionCard.svelte'
  import type { RowContext } from './context'
  import RowShell from './RowShell.svelte'

  let {
    item,
    ctx,
  }: { item: Extract<TimelineItem, { kind: 'approval' }>; ctx: RowContext } =
    $props()

  const Card = $derived(item.questions.length > 0 ? QuestionCard : ApprovalCard)
</script>

<RowShell id={item.id} {ctx} pop>
  <Card
    {item}
    nowMs={ctx.nowMs}
    timeLabel={formatThreadTime(item.atMs)}
    timeTitle={formatThreadTimeTitle(item.atMs)}
    onFocusComposer={ctx.onFocusComposer}
  />
</RowShell>
