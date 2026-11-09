<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { abortWorkspaceGit } from '$lib/workspace/git-actions'
  import type { GitConflictState } from '$lib/ipc/bindings'

  let {
    workspaceId,
    branch,
    base,
    conflict,
  }: {
    workspaceId: string
    branch: string
    base: string
    conflict: GitConflictState
  } = $props()
</script>

<div
  class="flex flex-col gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm"
  role="status"
  data-od-id="ws-git-conflict"
>
  <p class="font-medium">Merge from {base} into {branch} has conflicts</p>
  <ul class="list-inside list-disc font-mono text-xs text-muted-foreground">
    {#each conflict.paths as path (path)}
      <li class="truncate">{path}</li>
    {/each}
  </ul>
  <div class="flex flex-wrap gap-2">
    <Button
      size="sm"
      variant="secondary"
      onclick={() => void abortWorkspaceGit(workspaceId)}
    >
      Abort merge
    </Button>
  </div>
</div>
