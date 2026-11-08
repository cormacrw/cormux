<script lang="ts">
  import * as Alert from '$lib/components/ui/alert'
  import { Button } from '$lib/components/ui/button'
  import { commands } from '$lib/ipc'
  import { openSettings } from '$lib/settings/open'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import TriangleAlert from '@lucide/svelte/icons/triangle-alert'

  let { workspace }: { workspace: Workspace } = $props()

  let busy = $state(false)

  async function retry() {
    busy = true
    await commands.retryWorkspaceProvisioning(workspace.id)
    busy = false
  }

  async function skip() {
    busy = true
    await commands.skipWorkspaceProvisioningSetup(workspace.id)
    busy = false
  }

  function editSetup() {
    openSettings({ section: 'repos' })
  }

  const detail = $derived.by(() => {
    const failure = workspace.setupFailure
    if (failure) {
      return `“${failure.command}” exited with code ${failure.exitCode}. Fix the command or skip setup to start the agent anyway.`
    }
    return 'Worktree setup did not finish. Retry, skip remaining setup, or edit the repo’s setup commands.'
  })
</script>

<Alert.Root variant="destructive" class="border-destructive/40">
  <TriangleAlert class="size-4" />
  <Alert.Title>Worktree setup failed</Alert.Title>
  <Alert.Description class="grid gap-3">
    <p>{detail}</p>
    <div class="flex flex-wrap gap-2">
      <Button size="sm" disabled={busy} onclick={retry}>Retry</Button>
      <Button size="sm" variant="secondary" disabled={busy} onclick={skip}>
        Skip and continue
      </Button>
      <Button size="sm" variant="outline" disabled={busy} onclick={editSetup}>
        Edit setup commands
      </Button>
    </div>
  </Alert.Description>
</Alert.Root>
