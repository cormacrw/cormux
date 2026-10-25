<!--
  The New workspace dialog's base branch: a compact button that opens a
  searchable list, like the branch picker in the workspace header.
-->
<script lang="ts">
  import * as Command from '$lib/components/ui/command'
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu'
  import Check from '@lucide/svelte/icons/check'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import GitBranch from '@lucide/svelte/icons/git-branch'

  let {
    value = $bindable(''),
    branches,
    workspaceBranches = [],
    id,
    describedBy,
    invalid = false,
  }: {
    value?: string
    branches: string[]
    workspaceBranches?: string[]
    id: string
    describedBy?: string
    invalid?: boolean
  } = $props()

  let open = $state(false)
  let query = $state('')
  let searchEl = $state<HTMLInputElement | null>(null)

  // Type to search as soon as it opens.
  $effect(() => {
    if (!open) return
    requestAnimationFrame(() => searchEl?.focus())
  })

  // The chosen branch first, then the rest in the repo's order.
  const filtered = $derived.by(() => {
    const ordered = [
      ...(branches.includes(value) ? [value] : []),
      ...branches.filter((branch) => branch !== value),
    ]
    const q = query.trim().toLowerCase()
    return q
      ? ordered.filter((branch) => branch.toLowerCase().includes(q))
      : ordered
  })

  function meta(branch: string): string | null {
    if (branch === 'main') return 'default'
    if (branch === 'develop') return 'integration'
    if (branch === 'staging') return 'staging'
    if (workspaceBranches.includes(branch)) return 'workspace'
    return null
  }

  function pick(branch: string) {
    value = branch
    open = false
    query = ''
  }
</script>

<DropdownMenu.Root
  bind:open
  onOpenChange={(next) => {
    if (!next) query = ''
  }}
>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <button
        {...props}
        type="button"
        {id}
        class={[
          'glass-well flex h-8 max-w-[200px] min-w-0 items-center gap-1.5 rounded-[9px] px-2.5 font-mono text-xs outline-none focus-visible:ring-3 focus-visible:ring-ring/50',
          invalid && 'ring-2 ring-destructive/50',
        ]}
        aria-describedby={describedBy}
        title="Your new branch starts from here"
      >
        <GitBranch class="size-3.5 shrink-0 opacity-70" aria-hidden="true" />
        <span class="sr-only">Base branch</span>
        <span class="truncate">{value || 'Pick a branch'}</span>
        <ChevronDown class="size-3.5 shrink-0 opacity-60" aria-hidden="true" />
      </button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content
    align="start"
    class="glass-pop w-[min(100vw-2rem,20rem)] p-0"
    aria-label="Base branch"
  >
    <Command.Root
      shouldFilter={false}
      class="border-0 bg-transparent shadow-none"
    >
      <Command.Input
        bind:ref={searchEl}
        bind:value={query}
        placeholder="Search branches…"
        class="h-9"
      />
      <Command.List class="max-h-[280px]">
        {#if filtered.length === 0}
          <Command.Empty>No branches match</Command.Empty>
        {:else}
          <Command.Group>
            {#each filtered as branch (branch)}
              {@const label = meta(branch)}
              <Command.Item
                value={branch}
                onSelect={() => pick(branch)}
                class="flex gap-2 font-mono text-xs"
              >
                <span class="flex w-4 shrink-0 justify-center">
                  {#if branch === value}
                    <Check class="size-3.5" aria-hidden="true" />
                  {/if}
                </span>
                <span class="min-w-0 flex-1 truncate">{branch}</span>
                {#if label}
                  <span
                    class="shrink-0 text-[10px] tracking-wide text-muted-foreground uppercase"
                    >{label}</span
                  >
                {/if}
              </Command.Item>
            {/each}
          </Command.Group>
        {/if}
      </Command.List>
    </Command.Root>
  </DropdownMenu.Content>
</DropdownMenu.Root>
