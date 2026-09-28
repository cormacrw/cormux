<script lang="ts">
  import { tick } from 'svelte'
  import { Input } from '$lib/components/ui/input'
  import { commands } from '$lib/ipc'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { repos, settings } from '$lib/state'

  $effect(() => {
    const repoId = settings.focusRepoRunCommand
    if (!repoId) return
    void tick().then(() => {
      document.getElementById(`run-${repoId}`)?.focus()
      settings.focusRepoRunCommand = null
    })
  })

  async function saveRunCommand(repoId: string, value: string) {
    const trimmed = value.trim()
    const result = await commands.setRepoRunCommand({
      repoId,
      runCommand: trimmed || null,
    })
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    repos.hydrate(
      repos.items.map((repo) =>
        repo.id === repoId
          ? { ...repo, runCommand: trimmed || null }
          : repo,
      ),
    )
  }
</script>

<div class="mt-3 flex flex-col gap-2">
  {#each repos.items as repo (repo.id)}
    <details class="rounded-lg border border-border px-3 py-2" open={false}>
      <summary class="cursor-pointer text-sm font-medium">{repo.name}</summary>
      <div class="mt-2 space-y-1.5">
        <label class="text-xs text-muted-foreground" for="run-{repo.id}"
          >Run command</label
        >
        <Input
          id="run-{repo.id}"
          value={repo.runCommand ?? ''}
          placeholder="pnpm dev"
          class="font-mono text-xs"
          onchange={(event) =>
            saveRunCommand(repo.id, event.currentTarget.value)}
        />
      </div>
    </details>
  {:else}
    <p class="text-sm text-muted-foreground">No repositories registered yet.</p>
  {/each}
</div>
