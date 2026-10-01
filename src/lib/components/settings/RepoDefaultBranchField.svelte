<script lang="ts">
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle'
  import * as Select from '$lib/components/ui/select'
  import { commands } from '$lib/ipc'
  import type { RepoRecord } from '$lib/ipc/bindings'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { repos } from '$lib/state'

  let { repo, hintId }: { repo: RepoRecord; hintId: string } = $props()

  let branches = $state<string[]>([])
  let loading = $state(true)

  const current = $derived(repo.defaultBranch?.trim() || 'main')

  $effect(() => {
    void loadBranches(repo.id)
  })

  async function loadBranches(repoId: string) {
    loading = true
    const result = await commands.listRepoBranches(repoId)
    loading = false
    if (result.status === 'ok') branches = result.data.branches
  }

  async function choose(branch: string) {
    if (!branch || branch === current) return
    const result = await commands.setRepoDefaultBranch({
      repoId: repo.id,
      defaultBranch: branch,
    })
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    repos.hydrate(
      repos.items.map((row) =>
        row.id === repo.id ? { ...row, defaultBranch: branch } : row,
      ),
    )
  }
</script>

<div class="field space-y-1.5">
  <label class="text-sm font-medium" for="default-branch-{repo.id}">
    Default branch
  </label>
  {#if loading}
    <LoaderCircleIcon class="size-4 animate-spin text-muted-foreground" />
  {:else}
    <Select.Root
      type="single"
      value={current}
      onValueChange={(value) => void choose(value)}
    >
      <Select.Trigger
        id="default-branch-{repo.id}"
        class="w-[220px] font-mono text-xs"
        aria-describedby={hintId}
      >
        <span class="truncate">{current}</span>
      </Select.Trigger>
      <Select.Content>
        {#each branches.includes(current) ? branches : [current, ...branches] as branch (branch)}
          <Select.Item value={branch} class="font-mono text-xs"
            >{branch}</Select.Item
          >
        {/each}
      </Select.Content>
    </Select.Root>
  {/if}
  <p class="field-hint text-xs text-muted-foreground" id={hintId}>
    New workspaces branch from here. It stays in the repo's own checkout, so
    workspaces can't check it out; refresh it from the sidebar.
  </p>
</div>
