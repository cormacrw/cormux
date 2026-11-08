<script lang="ts">
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import * as Dialog from '$lib/components/ui/dialog/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import DialogShortcut from '$lib/components/shell/DialogShortcut.svelte'
  import { commands } from '$lib/ipc'
  import type { SubmitReviewVerdict } from '$lib/ipc/bindings'
  import { coreErrorText } from '$lib/core/core-error'
  import { onModEnter } from '$lib/keyboard/mod-enter'
  import { shellDialogs, workspaces } from '$lib/state'

  const workspaceId = $derived(
    shellDialogs.args('submitReview')?.workspaceId ?? null,
  )
  const open = $derived(workspaceId != null)
  const workspace = $derived(
    workspaceId ? workspaces.getById(workspaceId) : undefined,
  )

  let verdict = $state<SubmitReviewVerdict>('comment')
  let submitting = $state(false)
  let submitError = $state<string | null>(null)

  function reset() {
    verdict = 'comment'
    submitting = false
    submitError = null
  }

  function closeDialog() {
    shellDialogs.close('submitReview')
    reset()
  }

  async function submit() {
    if (!workspaceId || submitting) return
    submitting = true
    submitError = null
    const result = await commands.submitWorkspaceReview({
      workspaceId,
      verdict,
    })
    submitting = false
    if (result.status === 'error') {
      submitError = coreErrorText(result.error, 'Could not submit review')
      return
    }
    closeDialog()
  }

  const onKeydown = onModEnter(() => void submit())

  $effect(() => {
    if (open) reset()
  })
</script>

<Dialog.Root {open} onOpenChange={(next) => !next && closeDialog()}>
  <Dialog.Content class="max-w-md gap-0 p-0" onkeydown={onKeydown}>
    <Dialog.Header class="px-5 pt-5">
      <Dialog.Title>Submit review</Dialog.Title>
      {#if workspace?.prNumber}
        <p class="text-sm text-muted-foreground">
          Pull request #{workspace.prNumber}
        </p>
      {/if}
    </Dialog.Header>

    <div class="space-y-4 p-5">
      <fieldset class="space-y-2">
        <legend class="text-sm font-medium">Verdict</legend>
        <label class="flex items-center gap-2 text-sm">
          <input type="radio" bind:group={verdict} value="approve" />
          Approve
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input type="radio" bind:group={verdict} value="requestChanges" />
          Request changes
        </label>
        <label class="flex items-center gap-2 text-sm">
          <input type="radio" bind:group={verdict} value="comment" />
          Comment
        </label>
      </fieldset>
      <p class="text-xs text-muted-foreground">
        Open findings become line comments or review body text. Nothing is
        posted until you confirm.
      </p>
      {#if submitError}
        <p class="text-sm text-destructive">{submitError}</p>
      {/if}
    </div>

    <Dialog.Footer class="m-0 px-5 py-4">
      <Button
        size="xl"
        variant="outline"
        onclick={closeDialog}
        disabled={submitting}
      >
        Cancel
        <DialogShortcut keys="cancel" />
      </Button>
      <Button size="xl" onclick={() => void submit()} disabled={submitting}>
        {#if submitting}
          <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
          Posting…
        {:else}
          Submit review
          <DialogShortcut keys="submit" />
        {/if}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
