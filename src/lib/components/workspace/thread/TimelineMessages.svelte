<script lang="ts" module>
  import type { Component } from 'svelte'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import type { RowContext } from './rows/context'
  import ApprovalRow from './rows/ApprovalRow.svelte'
  import EventRow from './rows/EventRow.svelte'
  import FindingsRow from './rows/FindingsRow.svelte'
  import PlanRow from './rows/PlanRow.svelte'
  import ReplyRow from './rows/ReplyRow.svelte'
  import ThoughtRow from './rows/ThoughtRow.svelte'
  import ToolRunRow from './rows/ToolRunRow.svelte'
  import UserRow from './rows/UserRow.svelte'
  import WorkedRow from './rows/WorkedRow.svelte'

  type RowProps<Item> = { item: Item; ctx: RowContext }

  /** How each kind of item draws; a new kind doesn't compile until it has a row here. */
  const ROW = {
    user: UserRow,
    reply: ReplyRow,
    thought: ThoughtRow,
    toolRun: ToolRunRow,
    worked: WorkedRow,
    plan: PlanRow,
    approval: ApprovalRow,
    event: EventRow,
    findings: FindingsRow,
  } satisfies {
    [K in TimelineItem['kind']]: Component<
      RowProps<Extract<TimelineItem, { kind: K }>>
    >
  }

  function rowFor(item: TimelineItem) {
    return ROW[item.kind] as Component<RowProps<TimelineItem>>
  }
</script>

<script lang="ts">
  import { rowKey } from '$lib/thread/find'
  import type { TimelineRow } from '$lib/thread/timeline-types'
  import SpeakerRow from './rows/SpeakerRow.svelte'

  let { rows, ctx }: { rows: TimelineRow[]; ctx: RowContext } = $props()
</script>

{#each rows as row (rowKey(row))}
  {#if row.kind === 'speaker'}
    <SpeakerRow {row} {ctx} />
  {:else}
    {@const Row = rowFor(row.item)}
    <Row item={row.item} {ctx} />
  {/if}
{/each}
