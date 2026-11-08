<script lang="ts">
  import Check from '@lucide/svelte/icons/check'
  import List from '@lucide/svelte/icons/list'
  import * as Card from '$lib/components/ui/card'
  import {
    formatThreadTime,
    formatThreadTimeTitle,
  } from '$lib/thread/thread-time'
  import type { TimelineItem } from '$lib/thread/timeline-types'
  import { cn } from '$lib/utils'
  import type { RowContext } from './context'
  import RowShell from './RowShell.svelte'

  let {
    item,
    ctx,
  }: { item: Extract<TimelineItem, { kind: 'plan' }>; ctx: RowContext } =
    $props()

  const done = $derived(
    item.statuses.filter((status) => status === 'completed').length,
  )
  const clock = $derived(formatThreadTime(item.atMs))
</script>

<RowShell id={item.id} {ctx} pop>
  <Card.Root class="bg-popover">
    <Card.Header class="flex-row items-center gap-2 space-y-0 pb-2">
      <List class="size-4 text-muted-foreground" aria-hidden="true" />
      <Card.Title class="text-sm font-medium"
        >{item.statuses.some((status) => status !== 'pending')
          ? `Plan · ${done} of ${item.steps.length}`
          : 'Proposed plan'}</Card.Title
      >
      {#if clock}
        <span
          class="ml-auto text-xs text-muted-foreground"
          title={formatThreadTimeTitle(item.atMs)}>{clock}</span
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
</RowShell>
