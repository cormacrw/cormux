<script lang="ts" module>
  import type { DiffFile } from '$lib/ipc/bindings'

  /** One file, or every uncommitted change when `file` is null. */
  export type DiscardRequest = { file: DiffFile | null; count: number }
</script>

<script lang="ts">
  import * as AlertDialog from '$lib/components/ui/alert-dialog'
  import DialogShortcut from '$lib/components/shell/DialogShortcut.svelte'
  import { commands } from '$lib/ipc'
  import { inferFileStatus } from '$lib/changes/file-status'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'

  let {
    workspaceId,
    request,
    onClose,
  }: {
    workspaceId: string
    request: DiscardRequest | null
    onClose: () => void
  } = $props()

  // Held after close so the copy doesn't change while the dialog animates out.
  let shown = $state<DiscardRequest | null>(null)
  $effect(() => {
    if (request) shown = request
  })

  let busy = $state(false)

  function onOpenChange(next: boolean) {
    if (!next && !busy) onClose()
  }

  async function confirm() {
    if (!request || busy) return
    busy = true
    const { file } = request
    const paths = file
      ? file.oldPath
        ? [file.oldPath, file.path]
        : [file.path]
      : []
    const result = await commands.discardWorkspaceChanges(workspaceId, paths)
    busy = false
    onClose()
    if (result.status === 'error') toastCoreError(result.error)
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault()
      void confirm()
    }
  }
</script>

<AlertDialog.Root open={request != null} {onOpenChange}>
  <AlertDialog.Content
    class="max-w-md sm:max-w-md"
    onkeydown={onKeydown}
    data-od-id="discard-changes-dialog"
  >
    <AlertDialog.Header>
      <AlertDialog.Title>
        {shown?.file ? 'Discard changes to this file?' : 'Discard all changes?'}
      </AlertDialog.Title>
      <AlertDialog.Description>
        {#if shown?.file}
          {@const status = inferFileStatus(shown.file)}
          <code class="font-mono text-xs break-all">{shown.file.path}</code>
          {#if status === 'A'}
            is new, so it will be deleted.
          {:else if status === 'R'}
            moves back to
            <code class="font-mono text-xs break-all">{shown.file.oldPath}</code
            >, as it was at the last commit.
          {:else}
            goes back to how it was at the last commit.
          {/if}
        {:else}
          Every uncommitted change in this workspace ({shown?.count ?? 0}
          {shown?.count === 1 ? 'file' : 'files'}) goes back to the last commit,
          and new files are deleted.
        {/if}
        This can’t be undone.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel disabled={busy}
        >Cancel <DialogShortcut keys="cancel" /></AlertDialog.Cancel
      >
      <AlertDialog.Action
        variant="destructive"
        disabled={busy}
        onclick={(event) => {
          event.preventDefault()
          void confirm()
        }}
      >
        {#if busy}
          <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
          Discarding…
        {:else}
          Discard
          <DialogShortcut keys="submit" />
        {/if}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
