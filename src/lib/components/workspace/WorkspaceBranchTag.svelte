<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import {
    branchLockTooltip,
    branchPickerLocked,
    runningAgentCount,
  } from '$lib/workspace/running-agents'
  import type { Thread } from '$lib/state/threads.svelte'

  let {
    branch,
    base,
    behind,
    threads,
    provisioning,
  }: {
    branch: string
    base: string
    behind: number
    threads: Thread[]
    provisioning: boolean
  } = $props()

  const locked = $derived(
    provisioning || branchPickerLocked(threads),
  )
  const branchInfo = $derived(
    `Branched from ${base}${behind > 0 ? `, ${behind} commit${behind === 1 ? '' : 's'} behind` : ''}.`,
  )
  const lockHint = $derived(branchLockTooltip(runningAgentCount(threads)))
  const tooltip = $derived(
    locked && !provisioning
      ? `${branchInfo} ${lockHint}`
      : `${branchInfo} Click to switch branch.`,
  )

</script>

<Tooltip.Root>
  <Tooltip.Trigger>
    {#snippet child({ props })}
      <Button
        {...props}
        variant="outline"
        size="sm"
        disabled={locked}
        aria-haspopup="true"
        aria-expanded={false}
        aria-controls="ws-branch-menu"
        data-ws-focus="branch"
        class="max-w-[min(100%,14rem)] gap-1.5 font-mono text-xs"
        onclick={() => {
          /* branch picker lands in COR-16 branch epic */
        }}
      >
        <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
        <span class="sr-only">Branch</span>
        <span class="truncate">{branch}</span>
        {#if !locked}
          <ChevronDown class="size-3.5 shrink-0 opacity-70" aria-hidden="true" />
        {:else if runningAgentCount(threads) > 0}
          <span class="sr-only">, locked while agents are running</span>
        {/if}
      </Button>
    {/snippet}
  </Tooltip.Trigger>
  <Tooltip.Content>{tooltip}</Tooltip.Content>
</Tooltip.Root>
