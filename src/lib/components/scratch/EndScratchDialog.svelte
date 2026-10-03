<script lang="ts">
  import { tick } from 'svelte'
  import * as AlertDialog from '$lib/components/ui/alert-dialog'
  import DialogShortcut from '$lib/components/shell/DialogShortcut.svelte'
  import { commands, fetchSnapshot } from '$lib/ipc'
  import { coreErrorText } from '$lib/feedback/core-error'
  import { showToast } from '$lib/feedback/show-toast'
  import {
    app,
    hydrateFromSnapshot,
    repos,
    scratches,
    shellDialogs,
  } from '$lib/state'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import type { Scratch } from '$lib/state/scratches.svelte'

  const request = $derived(shellDialogs.endScratch)
  const open = $derived(request != null)
  // Held after close so the copy doesn't change while the dialog animates out.
  let scratch = $state<Scratch | null>(null)
  let fromHome = $state(false)
  $effect(() => {
    const target = request ? scratches.getById(request.scratchId) : undefined
    if (!request || !target) return
    scratch = target
    // Delete from a card, End from the scratch page. Same operation.
    fromHome = request.fromHome
  })
  const path = $derived(
    scratch ? (repos.getById(scratch.repoId)?.path ?? scratch.repoId) : '',
  )

  let busy = $state(false)

  function onOpenChange(next: boolean) {
    if (!next && !busy) shellDialogs.closeEndScratch()
  }

  async function confirm() {
    if (!request || !scratch || busy) return
    busy = true
    const { scratchId, fromHome: deleting } = request
    const { title } = scratch
    const index = scratches.items.findIndex((item) => item.id === scratchId)
    const result = await commands.endScratch(scratchId)
    busy = false
    shellDialogs.closeEndScratch()
    if (result.status === 'error') {
      showToast({
        tone: 'bad',
        parts: [
          {
            type: 'text',
            value: coreErrorText(result.error, 'Could not end the scratch'),
          },
        ],
      })
      return
    }
    showToast({
      tone: 'default',
      parts: [
        { type: 'text', value: `${deleting ? 'Deleted' : 'Ended'} ${title}` },
      ],
    })
    // Leave the page before refetching, so it never sits on a scratch that's gone.
    if (app.scratchId === scratchId) {
      app.openHomebase()
      hydrateFromSnapshot(await fetchSnapshot())
      return
    }
    hydrateFromSnapshot(await fetchSnapshot())
    // Homebase stays put: focus the card that slid into this slot, or New scratch.
    await tick()
    const next = scratches.items[Math.min(index, scratches.items.length - 1)]
    const target = next
      ? document.querySelector<HTMLElement>(
          `[data-action="open-scratch"][data-arg="${next.id}"]`,
        )
      : document.querySelector<HTMLElement>('[data-od-id="new-session"]')
    target?.focus()
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault()
      void confirm()
    }
  }
</script>

<AlertDialog.Root {open} {onOpenChange}>
  <AlertDialog.Content class="max-w-md sm:max-w-md" onkeydown={onKeydown}>
    <AlertDialog.Header>
      <AlertDialog.Title>
        {fromHome ? 'Delete this scratch?' : 'End this scratch?'}
      </AlertDialog.Title>
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
          {fromHome ? 'Deleting…' : 'Ending…'}
        {:else}
          {fromHome ? 'Delete scratch' : 'End scratch'}
          <DialogShortcut keys="submit" />
        {/if}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
