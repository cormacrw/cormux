<script lang="ts">
  import { navigate } from '$lib/core/nav.svelte'
  import { tick } from 'svelte'
  import * as AlertDialog from '$lib/components/ui/alert-dialog'
  import DialogShortcut from '$lib/primitives/DialogShortcut.svelte'
  import { commands } from '$lib/ipc'
  import type { TeardownPreview } from '$lib/ipc/bindings'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { shellDialogs, workspaces } from '$lib/state'
  import { onModEnter } from '$lib/keyboard/mod-enter'
  import AlertTriangle from '@lucide/svelte/icons/alert-triangle'
  import Trash2 from '@lucide/svelte/icons/trash-2'

  const workspaceId = $derived(
    shellDialogs.args('teardown')?.workspaceId ?? null,
  )

  let preview = $state<TeardownPreview | null>(null)
  // Waits for the preview: a clean worktree never shows the dialog at all.
  const open = $derived(workspaceId != null && preview != null)

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
    preview = null
    const result = await commands.getTeardownPreview(id)
    if (workspaceId !== id) return
    if (result.status === 'error') {
      toastCoreError(result.error)
      shellDialogs.close('teardown')
      return
    }
    deleteBranch = result.data.deleteBranchDefault
    if (result.data.dataLoss.uncommittedFiles === 0) {
      confirmTeardown()
      return
    }
    preview = result.data
    await tick()
    confirmRef?.focus()
  }

  function onOpenChange(next: boolean) {
    if (!next) shellDialogs.close('teardown')
  }

  // Close immediately and let the teardown run in the background: the sidebar
  // drops the workspace and its row shows Deleting until the core
  // finishes. A failure toasts here; success needs no toast, the row just goes.
  function confirmTeardown() {
    if (!workspaceId) return
    const tornId = workspaceId
    workspaces.markDeleting(tornId)
    shellDialogs.close('teardown')
    navigate({ view: 'newThread' })
    void commands
      .teardownWorkspace({ workspaceId: tornId, deleteBranch })
      .then((result) => {
        if (result.status === 'error') {
          workspaces.clearDeleting(tornId)
          toastCoreError(result.error)
        }
      })
  }

  const onKeydown = onModEnter(() => {
    if (preview) confirmTeardown()
  })

  const description = $derived.by(() => {
    if (!preview) return ''
    const appPart = preview.appRunning ? ' and the running app' : ''
    return `Stops ${preview.engineLabel}${appPart}, then deletes ${preview.worktreePath}.`
  })
</script>

<AlertDialog.Root {open} {onOpenChange}>
  <AlertDialog.Content class="max-w-md sm:max-w-md" onkeydown={onKeydown}>
    <AlertDialog.Header>
      <AlertDialog.Title>Teardown & delete worktree?</AlertDialog.Title>
      <AlertDialog.Description>{description}</AlertDialog.Description>
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
        disabled={!preview}
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
