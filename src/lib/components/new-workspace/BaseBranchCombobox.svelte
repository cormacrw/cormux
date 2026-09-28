<script lang="ts">
  import { tick } from 'svelte'
  import SearchIcon from '@lucide/svelte/icons/search'
  import * as Command from '$lib/components/ui/command/index.js'
  import { Input } from '$lib/components/ui/input/index.js'
  import { cn } from '$lib/utils.js'

  type BranchMeta = 'default' | 'integration' | 'staging' | 'workspace'

  let {
    value = $bindable(''),
    branches,
    workspaceBranches = [],
    error = $bindable<string | null>(null),
    id,
    hintId,
    errorId,
    disabled = false,
  }: {
    value?: string
    branches: string[]
    workspaceBranches?: string[]
    error?: string | null
    id: string
    hintId: string
    errorId: string
    disabled?: boolean
  } = $props()

  let open = $state(false)
  let query = $state('')
  let activeIndex = $state(0)
  let inputEl = $state<HTMLInputElement | null>(null)
  const listId = $derived(`${id}-list`)

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase()
    if (!q) return branches
    return branches.filter((branch) => branch.toLowerCase().includes(q))
  })

  function metaFor(branch: string): BranchMeta | null {
    if (branch === 'main') return 'default'
    if (branch === 'develop') return 'integration'
    if (branch === 'staging') return 'staging'
    if (workspaceBranches.includes(branch)) return 'workspace'
    return null
  }

  function metaLabel(meta: BranchMeta | null): string | null {
    if (!meta) return null
    if (meta === 'default') return 'default'
    if (meta === 'integration') return 'integration'
    if (meta === 'staging') return 'deploys to staging'
    return 'workspace'
  }

  function highlight(branch: string, q: string) {
    if (!q) return [{ text: branch, match: false }]
    const lower = branch.toLowerCase()
    const index = lower.indexOf(q.toLowerCase())
    if (index < 0) return [{ text: branch, match: false }]
    return [
      { text: branch.slice(0, index), match: false },
      { text: branch.slice(index, index + q.length), match: true },
      { text: branch.slice(index + q.length), match: false },
    ].filter((part) => part.text.length > 0)
  }

  function pick(branch: string) {
    value = branch
    query = branch
    error = null
    open = false
  }

  export function closeList() {
    open = false
  }

  export function isOpen() {
    return open
  }

  async function onFocus() {
    query = value
    activeIndex = Math.max(0, branches.indexOf(value))
    open = true
    await tick()
    inputEl?.select()
  }

  function onInput() {
    value = query
    activeIndex = 0
    error = null
    open = true
  }

  function onBlur() {
    window.setTimeout(() => {
      open = false
      const trimmed = value.trim()
      if (trimmed && !branches.includes(trimmed)) {
        error = `No branch called “${trimmed}”. Pick one from the list.`
      }
    }, 120)
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      if (!open) {
        open = true
        return
      }
      activeIndex = Math.min(filtered.length - 1, activeIndex + 1)
      return
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      if (open) activeIndex = Math.max(0, activeIndex - 1)
      return
    }
    if (event.key === 'Enter' && open && filtered.length > 0) {
      event.preventDefault()
      const choice = filtered[activeIndex] ?? filtered[0]
      if (choice) pick(choice)
      return
    }
    if (event.key === 'Escape' && open) {
      event.preventDefault()
      event.stopPropagation()
      open = false
      return
    }
    if (
      event.key === 'Tab' &&
      open &&
      filtered.length > 0 &&
      value.trim() &&
      !branches.includes(value.trim())
    ) {
      const choice = filtered[activeIndex] ?? filtered[0]
      if (choice) pick(choice)
    }
  }

  $effect(() => {
    if (activeIndex >= filtered.length) activeIndex = 0
  })

  $effect(() => {
    if (!open) query = value
  })
</script>

<div class="relative">
  <div class="relative">
    <SearchIcon
      class="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground"
      aria-hidden="true"
    />
    <Input
      bind:ref={inputEl}
      {id}
      bind:value={query}
      class="font-mono pl-8"
      spellcheck={false}
      role="combobox"
      aria-expanded={open}
      aria-controls={listId}
      aria-activedescendant={open && filtered[activeIndex]
        ? `${id}-opt-${activeIndex}`
        : undefined}
      aria-invalid={error ? 'true' : undefined}
      aria-describedby={`${hintId} ${errorId}`}
      {disabled}
      onfocus={onFocus}
      oninput={onInput}
      onblur={onBlur}
      onkeydown={onKeydown}
    />
  </div>
  {#if open}
    <div
      id={listId}
      class="bg-popover text-popover-foreground absolute z-50 mt-1 max-h-56 w-full overflow-hidden rounded-lg border shadow-md"
      role="listbox"
    >
      <Command.Root
        shouldFilter={false}
        class="rounded-none border-0 shadow-none"
      >
        <Command.List class="max-h-56">
          {#if filtered.length === 0}
            <div
              class="px-3 py-4 text-sm text-muted-foreground"
              role="presentation"
            >
              No branches match “{query.trim()}”
            </div>
          {:else}
            {#each filtered as branch, index (branch)}
              {@const meta = metaLabel(metaFor(branch))}
              <button
                type="button"
                id="{id}-opt-{index}"
                role="option"
                aria-selected={index === activeIndex}
                class={cn(
                  'flex w-full items-center justify-between gap-2 px-3 py-2 text-left font-mono text-sm hover:bg-accent',
                  index === activeIndex && 'bg-accent',
                )}
                onmousedown={(event) => {
                  event.preventDefault()
                  pick(branch)
                }}
              >
                <span>
                  {#each highlight(branch, query.trim()) as part, i (i)}
                    {#if part.match}
                      <mark class="rounded bg-primary/20 px-0.5"
                        >{part.text}</mark
                      >
                    {:else}
                      {part.text}
                    {/if}
                  {/each}
                </span>
                {#if meta}
                  <span class="text-xs text-muted-foreground">{meta}</span>
                {/if}
              </button>
            {/each}
          {/if}
        </Command.List>
      </Command.Root>
    </div>
  {/if}
</div>
