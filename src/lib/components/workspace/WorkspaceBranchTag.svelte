<script lang="ts">
  import { tick } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import * as Command from '$lib/components/ui/command'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import Check from '@lucide/svelte/icons/check'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import Plus from '@lucide/svelte/icons/plus'
  import {
    branchLockTooltip,
    branchPickerLocked,
    runningAgentCount,
  } from '$lib/workspace/running-agents'
  import {
    branchMetaLabel,
    buildBranchPickerList,
  } from '$lib/workspace/branch-picker'
  import { switchWorkspaceBranch, createWorkspaceBranch } from '$lib/workspace/wire-git-workspace'
  import { commands } from '$lib/ipc'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { workspaceRecords } from '$lib/state'
  import type { Thread } from '$lib/state/threads.svelte'

  let {
    workspaceId,
    repoId,
    branch,
    base,
    behind,
    ahead,
    threads,
    provisioning,
  }: {
    workspaceId: string
    repoId: string
    branch: string
    base: string
    behind: number
    ahead: number
    threads: Thread[]
    provisioning: boolean
  } = $props()

  let open = $state(false)
  let query = $state('')
  let repoBranches = $state<string[]>([])
  let loadingBranches = $state(false)
  let triggerEl: HTMLButtonElement | undefined = $state()
  let newBranchName = $state('')

  const locked = $derived(provisioning || branchPickerLocked(threads))
  const branchInfo = $derived.by(() => {
    let text = `Branched from ${base}`
    if (behind > 0) {
      text += `, ${behind} commit${behind === 1 ? '' : 's'} behind`
    }
    if (ahead > 0) {
      text += `, ${ahead} commit${ahead === 1 ? '' : 's'} ahead`
    }
    return `${text}.`
  })
  const lockHint = $derived(branchLockTooltip(runningAgentCount(threads)))
  const tooltip = $derived(
    locked && !provisioning
      ? `${branchInfo} ${lockHint}`
      : `${branchInfo} Click to switch branch.`,
  )

  const otherWorkspaceBranches = $derived(
    workspaceRecords.records
      .filter(
        (record) =>
          record.repoId === repoId &&
          record.id !== workspaceId &&
          record.branch,
      )
      .map((record) => record.branch),
  )

  const pickerItems = $derived(
    buildBranchPickerList({
      current: branch,
      repoBranches,
      otherWorkspaceBranches,
    }),
  )

  const filteredItems = $derived.by(() => {
    const q = query.trim().toLowerCase()
    if (!q) return pickerItems
    return pickerItems.filter((item) =>
      item.name.toLowerCase().includes(q),
    )
  })

  async function loadBranches() {
    loadingBranches = true
    const result = await commands.listRepoBranches(repoId)
    loadingBranches = false
    if (result.status === 'ok') {
      repoBranches = result.data.branches
    } else {
      toastCoreError(result.error)
    }
  }

  $effect(() => {
    if (open && repoId) {
      void loadBranches()
    }
  })

  async function pickBranch(name: string) {
    open = false
    query = ''
    await tick()
    triggerEl?.focus()
    if (name === branch) return
    await switchWorkspaceBranch(workspaceId, name)
  }

  async function submitNewBranch() {
    const name = newBranchName.trim()
    if (!name) return
    open = false
    newBranchName = ''
    query = ''
    await tick()
    triggerEl?.focus()
    await createWorkspaceBranch(workspaceId, name)
  }
</script>

{#if locked}
  <Tooltip.Root>
    <Tooltip.Trigger>
      {#snippet child({ props })}
        <Button
          {...props}
          variant="outline"
          size="sm"
          disabled
          aria-disabled="true"
          data-ws-focus="branch"
          data-od-id="ws-branch"
          class="max-w-[min(100%,14rem)] gap-1.5 font-mono text-xs"
        >
          <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
          <span class="sr-only">Branch</span>
          <span class="truncate">{branch}</span>
          {#if runningAgentCount(threads) > 0}
            <span class="sr-only">, locked while agents are running</span>
          {/if}
        </Button>
      {/snippet}
    </Tooltip.Trigger>
    <Tooltip.Content>{tooltip}</Tooltip.Content>
  </Tooltip.Root>
{:else}
  <DropdownMenu.Root bind:open>
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <DropdownMenu.Trigger>
            {#snippet child({ props: menuProps })}
              <Button
                {...props}
                {...menuProps}
                bind:ref={triggerEl}
                variant="outline"
                size="sm"
                aria-haspopup="true"
                aria-expanded={open}
                aria-controls="ws-branch-menu"
                data-ws-focus="branch"
                data-od-id="ws-branch"
                class="max-w-[min(100%,14rem)] gap-1.5 font-mono text-xs"
              >
                <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
                <span class="sr-only">Branch</span>
                <span class="truncate">{branch}</span>
                <ChevronDown
                  class="size-3.5 shrink-0 opacity-70"
                  aria-hidden="true"
                />
              </Button>
            {/snippet}
          </DropdownMenu.Trigger>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content>{tooltip}</Tooltip.Content>
    </Tooltip.Root>
    <DropdownMenu.Content
      id="ws-branch-menu"
      align="start"
      class="w-[min(100vw-2rem,20rem)] p-0"
      role="menu"
      aria-label="Switch branch"
    >
      <DropdownMenu.Label class="px-3 py-2 text-xs font-medium">
        Switch branch
      </DropdownMenu.Label>
      <Command.Root shouldFilter={false} class="border-0 shadow-none">
        <Command.Input
          bind:value={query}
          placeholder="Search branches…"
          class="h-9"
        />
        <Command.List class="max-h-[280px] min-w-[280px]">
          {#if loadingBranches}
            <Command.Empty>Loading branches…</Command.Empty>
          {:else if filteredItems.length === 0}
            <Command.Empty>No branches match</Command.Empty>
          {:else}
            <Command.Group>
              {#each filteredItems as item (item.name)}
                {@const meta = branchMetaLabel(item.meta)}
                <Command.Item
                  value={item.name}
                  disabled={item.disabled}
                  role="menuitemradio"
                  aria-checked={item.checked}
                  aria-disabled={item.disabled}
                  title={item.meta === 'inOtherWorkspace'
                    ? "Checked out in another workspace's worktree"
                    : undefined}
                  onSelect={() => {
                    if (!item.disabled) void pickBranch(item.name)
                  }}
                  class="flex gap-2 font-mono text-xs"
                >
                  <span class="flex w-4 shrink-0 justify-center">
                    {#if item.checked}
                      <Check class="size-3.5" aria-hidden="true" />
                    {/if}
                  </span>
                  <span class="min-w-0 flex-1 truncate">{item.name}</span>
                  {#if meta}
                    <span
                      class="shrink-0 text-[10px] uppercase tracking-wide text-muted-foreground"
                    >
                      {meta}
                    </span>
                  {/if}
                </Command.Item>
              {/each}
            </Command.Group>
          {/if}
        </Command.List>
      </Command.Root>
      <div class="flex gap-2 border-t border-border/60 p-2">
        <input
          type="text"
          bind:value={newBranchName}
          placeholder="New branch name"
          class="min-w-0 flex-1 rounded-md border border-input bg-background px-2 py-1 font-mono text-xs"
          onkeydown={(event) => {
            if (event.key === 'Enter') void submitNewBranch()
          }}
        />
        <Button
          size="sm"
          variant="secondary"
          class="shrink-0 gap-1"
          disabled={!newBranchName.trim()}
          onclick={() => void submitNewBranch()}
        >
          <Plus class="size-3.5" aria-hidden="true" />
          Create
        </Button>
      </div>
    </DropdownMenu.Content>
  </DropdownMenu.Root>
{/if}
