<script lang="ts">
  import { refreshSnapshot } from '$lib/core/sync'
  import { navigate } from '$lib/core/nav.svelte'
  import * as AlertDialog from '$lib/components/ui/alert-dialog'
  import DialogShortcut from '$lib/components/shell/DialogShortcut.svelte'
  import { commands } from '$lib/ipc'
  import { showToast } from '$lib/feedback/show-toast'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { onModEnter } from '$lib/keyboard/mod-enter'
  import { repos, scratches, shellDialogs } from '$lib/state'
  import { nav } from '$lib/core/nav.svelte'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import type { Scratch } from '$lib/state/scratches.svelte'

  const request = $derived(shellDialogs.args('endScratch'))
  const open = $derived(request != null)
  // Held after close so the copy doesn't change while the dialog animates out.
  let scratch = $state<Scratch | null>(null)
  $effect(() => {
    const target = request ? scratches.getById(request.scratchId) : undefined
    if (!request || !target) return
    scratch = target
  })
  const path = $derived(
    scratch ? (repos.getById(scratch.repoId)?.path ?? scratch.repoId) : '',
  )

  let busy = $state(false)

  function onOpenChange(next: boolean) {
    if (!next && !busy) shellDialogs.close('endScratch')
  }

  async function confirm() {
    if (!request || !scratch || busy) return
    busy = true
    const { scratchId } = request
    const { title } = scratch
    const result = await commands.endScratch(scratchId)
    busy = false
    shellDialogs.close('endScratch')
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    showToast({
      tone: 'default',
      parts: [{ type: 'text', value: `Deleted ${title}` }],
    })
    // Leave the page before refetching, so it never sits on a scratch that's gone.
    if (nav.scratchId === scratchId) navigate({ view: 'newThread' })
    await refreshSnapshot()
  }

  const onKeydown = onModEnter(() => void confirm())
</script>

<AlertDialog.Root {open} {onOpenChange}>
  <AlertDialog.Content class="max-w-md sm:max-w-md" onkeydown={onKeydown}>
    <AlertDialog.Header>
      <AlertDialog.Title>Delete this scratch?</AlertDialog.Title>
      <AlertDialog.Description>
        Discards the conversation for {scratch?.title}.
        <code class="font-mono text-xs">{path}</code> is not changed.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel disabled={busy}
        >Cancel <DialogShortcut keys="cancel" /></AlertDialog.Cancel
      >
      <AlertDialog.Action
        variant="destructive"
        disabled={busy || !scratch}
        onclick={(event) => {
          event.preventDefault()
          void confirm()
        }}
      >
        {#if busy}
          <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
          Deleting…
        {:else}
          Delete scratch
          <DialogShortcut keys="submit" />
        {/if}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
