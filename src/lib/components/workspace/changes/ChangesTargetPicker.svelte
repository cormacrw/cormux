<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import * as Command from '$lib/components/ui/command'
  import Check from '@lucide/svelte/icons/check'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import GitCompare from '@lucide/svelte/icons/git-compare'
  import { commands } from '$lib/ipc'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { workspaceDiff, workspaceRecords } from '$lib/state'

  let {
    workspaceId,
    branch,
  }: {
    workspaceId: string
    branch: string
  } = $props()

  const UNCOMMITTED = 'Uncommitted'

  let open = $state(false)
  let query = $state('')
  let repoBranches = $state<string[]>([])
  let loadingBranches = $state(false)

  const repoId = $derived(workspaceRecords.getRecord(workspaceId)?.repoId ?? '')
  const base = $derived(workspaceDiff.base(workspaceId))
  // While a retarget loads, the label shows where it is headed.
  const pending = $derived(workspaceDiff.pending(workspaceId))
  const shownBase = $derived(pending?.retarget ? pending.base : base)

  const filteredBranches = $derived.by(() => {
    const q = query.trim().toLowerCase()
    const others = repoBranches.filter((name) => name !== branch)
    return q ? others.filter((name) => name.toLowerCase().includes(q)) : others
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
    if (open && repoId) void loadBranches()
  })

  async function pick(next: string | null) {
    open = false
    query = ''
    if (next === base) return
    await workspaceDiff.fetch(
      workspaceId,
      async () => {
        const result = await commands.setWorkspaceDiffBase(workspaceId, next)
        if (result.status === 'error') toastCoreError(result.error)
        return result.status === 'ok'
      },
      { base: next },
    )
  }
</script>

<DropdownMenu.Root bind:open>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <Button
        {...props}
        variant="outline"
        size="sm"
        aria-haspopup="true"
        aria-expanded={open}
        data-od-id="changes-target"
        title={shownBase
          ? `Committed changes on ${branch} since it left ${shownBase}`
          : 'Uncommitted changes vs HEAD'}
        class="h-6 max-w-[14rem] min-w-0 gap-1 px-1.5 font-mono text-xs"
      >
        <GitCompare class="size-3.5 shrink-0" aria-hidden="true" />
        <span class="sr-only">Compare against</span>
        <span class="truncate"
          >{shownBase ? `vs ${shownBase}` : UNCOMMITTED}</span
        >
        <ChevronDown class="size-3.5 shrink-0 opacity-70" aria-hidden="true" />
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content
    align="start"
    class="w-[min(100vw-2rem,20rem)] p-0"
    aria-label="Compare against"
  >
    <DropdownMenu.Label class="px-3 py-2 text-xs font-medium">
      Compare against
    </DropdownMenu.Label>
    <Command.Root shouldFilter={false} class="border-0 shadow-none">
      <Command.Input
        bind:value={query}
        placeholder="Search branches…"
        class="h-9"
      />
      <Command.List class="max-h-[280px] min-w-[280px]">
        {#if !query.trim()}
          <Command.Group>
            <Command.Item
              value={UNCOMMITTED}
              role="menuitemradio"
              aria-checked={base === null}
              onSelect={() => void pick(null)}
              class="flex gap-2 text-xs"
            >
              <span class="flex w-4 shrink-0 justify-center">
                {#if base === null}<Check
                    class="size-3.5"
                    aria-hidden="true"
                  />{/if}
              </span>
              <span class="min-w-0 flex-1 truncate">Uncommitted changes</span>
            </Command.Item>
          </Command.Group>
        {/if}
        {#if loadingBranches}
          <Command.Empty>Loading branches…</Command.Empty>
        {:else if filteredBranches.length === 0}
          <Command.Empty>No branches match</Command.Empty>
        {:else}
          <Command.Group heading="Branch">
            {#each filteredBranches as name (name)}
              <Command.Item
                value={name}
                role="menuitemradio"
                aria-checked={base === name}
                onSelect={() => void pick(name)}
                class="flex gap-2 font-mono text-xs"
              >
                <span class="flex w-4 shrink-0 justify-center">
                  {#if base === name}<Check
                      class="size-3.5"
                      aria-hidden="true"
                    />{/if}
                </span>
                <span class="min-w-0 flex-1 truncate">{name}</span>
              </Command.Item>
            {/each}
          </Command.Group>
        {/if}
      </Command.List>
    </Command.Root>
  </DropdownMenu.Content>
</DropdownMenu.Root>
