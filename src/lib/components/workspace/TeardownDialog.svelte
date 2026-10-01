<script lang="ts">
  import { tick } from 'svelte'
  import * as AlertDialog from '$lib/components/ui/alert-dialog'
  import DialogShortcut from '$lib/components/shell/DialogShortcut.svelte'
  import { commands } from '$lib/ipc'
  import type { TeardownPreview } from '$lib/ipc/bindings'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { fetchSnapshot } from '$lib/ipc'
  import { app, hydrateFromSnapshot, shellDialogs } from '$lib/state'
  import AlertTriangle from '@lucide/svelte/icons/alert-triangle'
  import Trash2 from '@lucide/svelte/icons/trash-2'

  const workspaceId = $derived(shellDialogs.teardownWorkspaceId)
  const open = $derived(workspaceId != null)

  let preview = $state<TeardownPreview | null>(null)
  let loadingPreview = $state(false)
  let deleteBranch = $state(true)
  let confirmRef = $state<HTMLButtonElement | null>(null)

  $effect(() => {
    if (!workspaceId) {
      preview = null
      return
    }
    void loadPreview(workspaceId)
  })

  async function loadPreview(id: string) {
    loadingPreview = true
    preview = null
    const result = await commands.getTeardownPreview(id)
    loadingPreview = false
    if (result.status === 'error') {
      toastCoreError(JSON.stringify(result.error))
      shellDialogs.closeTeardown()
      return
    }
    preview = result.data
    deleteBranch = result.data.deleteBranchDefault
    await tick()
    confirmRef?.focus()
  }

  function onOpenChange(next: boolean) {
    if (!next) shellDialogs.closeTeardown()
  }

  // Close immediately and let the teardown run in the background; the core
  // toasts on success and we toast here on failure.
  function confirmTeardown() {
    if (!workspaceId) return
    const tornId = workspaceId
    shellDialogs.closeTeardown()
    if (app.workspaceId === tornId) {
      app.openHomebase()
    }
    void commands
      .teardownWorkspace({ workspaceId: tornId, deleteBranch })
      .then(async (result) => {
        if (result.status === 'error') {
          toastCoreError(JSON.stringify(result.error))
        }
        hydrateFromSnapshot(await fetchSnapshot())
      })
  }

  function onKeydown(event: KeyboardEvent) {
    if (
      event.key === 'Enter' &&
      (event.metaKey || event.ctrlKey) &&
      preview &&
      !loadingPreview
    ) {
      event.preventDefault()
      confirmTeardown()
    }
  }

  const description = $derived.by(() => {
    if (!preview) return ''
    const appPart = preview.appRunning ? ' and the running app' : ''
    return `Stops ${preview.engineLabel}${appPart}, then deletes ${preview.worktreePath}. Homebase is not affected.`
  })
</script>

<AlertDialog.Root {open} {onOpenChange}>
  <AlertDialog.Content class="max-w-md sm:max-w-md" onkeydown={onKeydown}>
    <AlertDialog.Header>
      <AlertDialog.Title>Teardown & delete worktree?</AlertDialog.Title>
      <AlertDialog.Description>
        {#if loadingPreview}
          Loading…
        {:else}
          {description}
        {/if}
      </AlertDialog.Description>
    </AlertDialog.Header>

    {#if preview}
      <label class="flex cursor-pointer items-start gap-2 text-sm">
        <input
          type="checkbox"
          class="mt-0.5 size-4 rounded border border-input"
          bind:checked={deleteBranch}
        />
        <span>
          Also delete local branch <code class="font-mono text-xs"
            >{preview.branch}</code
          >
        </span>
      </label>

      {#if preview.dataLoss.warning}
        <div
          class="flex gap-2 rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive"
          role="alert"
        >
          <AlertTriangle class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
          <span>{preview.dataLoss.warning}</span>
        </div>
      {/if}
    {/if}

    <AlertDialog.Footer>
      <AlertDialog.Cancel
        >Cancel <DialogShortcut keys="cancel" /></AlertDialog.Cancel
      >
      <AlertDialog.Action
        bind:ref={confirmRef}
        variant="destructive"
        disabled={loadingPreview || !preview}
        onclick={(event) => {
          event.preventDefault()
          confirmTeardown()
        }}
      >
        <Trash2 class="size-4" aria-hidden="true" />
        Teardown
        <DialogShortcut keys="submit" />
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
