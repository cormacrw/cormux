<script lang="ts">
  import { tick } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import * as Command from '$lib/components/ui/command'
  import Check from '@lucide/svelte/icons/check'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import Plus from '@lucide/svelte/icons/plus'
  import {
    branchPickerLocked,
    runningAgentCount,
  } from '$lib/workspace/running-agents'
  import {
    branchMetaLabel,
    buildBranchPickerList,
  } from '$lib/workspace/branch-picker'
  import {
    switchWorkspaceBranch,
    createWorkspaceBranch,
  } from '$lib/workspace/wire-git-workspace'
  import { commands } from '$lib/ipc'
  import { isHeaderShortcut } from '$lib/keyboard/header-shortcuts'
  import { Kbd } from '$lib/components/ui/kbd'
  import CommandIcon from '@lucide/svelte/icons/command'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { repos, workspaceRecords } from '$lib/state'
  import type { Thread } from '$lib/state/threads.svelte'

  let {
    workspaceId,
    repoId,
    branch,
    threads,
    provisioning,
  }: {
    workspaceId: string
    repoId: string
    branch: string
    threads: Thread[]
    provisioning: boolean
  } = $props()

  let open = $state(false)
  let query = $state('')
  let repoBranches = $state<string[]>([])
  let loadingBranches = $state(false)
  let triggerEl = $state<HTMLButtonElement | null>(null)
  let newBranchName = $state('')

  const locked = $derived(provisioning || branchPickerLocked(threads))

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
      defaultBranch: repos.getById(repoId)?.defaultBranch?.trim() || 'main',
    }),
  )

  const filteredItems = $derived.by(() => {
    const q = query.trim().toLowerCase()
    if (!q) return pickerItems
    return pickerItems.filter((item) => item.name.toLowerCase().includes(q))
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

  function onKeydown(event: KeyboardEvent) {
    if (!isHeaderShortcut(event, 'b')) return
    event.preventDefault()
    if (!locked) open = !open
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if locked}
  <Button
    variant="outline"
    size="sm"
    disabled
    aria-disabled="true"
    data-ws-focus="branch"
    data-od-id="ws-branch"
    class="h-8 max-w-[min(100%,16rem)] shrink-0 gap-1.5 !rounded-full !bg-custard px-3 font-display text-sm font-extrabold !text-cocoa"
  >
    <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
    <span class="sr-only">Branch</span>
    <span class="truncate">{branch}</span>
    {#if runningAgentCount(threads) > 0}
      <span class="sr-only">, locked while agents are running</span>
    {/if}
  </Button>
{:else}
  <DropdownMenu.Root bind:open>
    <DropdownMenu.Trigger>
      {#snippet child({ props })}
        <Button
          {...props}
          bind:ref={triggerEl}
          variant="outline"
          size="sm"
          aria-haspopup="true"
          aria-expanded={open}
          aria-controls="ws-branch-menu"
          aria-keyshortcuts="Meta+B"
          data-ws-focus="branch"
          data-od-id="ws-branch"
          class="h-8 max-w-[min(100%,16rem)] shrink-0 gap-1.5 !rounded-full !bg-custard px-3 font-display text-sm font-extrabold !text-cocoa"
        >
          <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
          <span class="sr-only">Branch</span>
          <span class="truncate">{branch}</span>
          <Kbd
            class="h-4 min-w-4 shrink-0 gap-0.5 px-1 text-[10px]"
            aria-hidden="true"><CommandIcon class="size-2.5" />B</Kbd
          >
          <ChevronDown
            class="size-3.5 shrink-0 opacity-70"
            aria-hidden="true"
          />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <DropdownMenu.Content
      id="ws-branch-menu"
      align="start"
      class="w-[min(100vw-2rem,22rem)] p-2"
      role="menu"
      aria-label="Switch branch"
    >
      <DropdownMenu.Label class="eyebrow px-3 pt-1 pb-1 text-muted-foreground">
        Switch branch
      </DropdownMenu.Label>
      <Command.Root shouldFilter={false} class="bg-transparent p-0 shadow-none">
        <Command.Input
          bind:value={query}
          placeholder="Search branches…"
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
                  onSelect={() => {
                    if (!item.disabled) void pickBranch(item.name)
                  }}
                  class="font-mono text-[13px] font-semibold"
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
      <div class="mt-1 flex gap-2 border-t border-border p-2 pt-3">
        <input
          type="text"
          bind:value={newBranchName}
          placeholder="New branch name"
          class="sunken min-w-0 flex-1 px-3 py-1.5 font-mono text-[13px]"
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
