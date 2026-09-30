<script lang="ts">
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import * as Dialog from '$lib/components/ui/dialog/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import { commands } from '$lib/ipc'
  import type {
    Error as CoreError,
    SubmitReviewVerdict,
  } from '$lib/ipc/bindings'
  import { fetchSnapshot } from '$lib/ipc'
  import { dismissOpenPopover } from '$lib/keyboard/global-shortcuts'
  import { hydrateFromSnapshot, shellDialogs, workspaces } from '$lib/state'

  const workspaceId = $derived(shellDialogs.submitReviewWorkspaceId)
  const open = $derived(workspaceId != null)
  const workspace = $derived(
    workspaceId ? workspaces.getById(workspaceId) : undefined,
  )

  let verdict = $state<SubmitReviewVerdict>('comment')
  let submitting = $state(false)
  let submitError = $state<string | null>(null)

  function coreErrorMessage(error: CoreError): string {
    if (
      error.kind === 'Git' ||
      error.kind === 'Workspace' ||
      error.kind === 'Github' ||
      error.kind === 'Llm'
    ) {
      return error.message
    }
    return 'Could not submit review'
  }

  function reset() {
    verdict = 'comment'
    submitting = false
    submitError = null
  }

  function closeDialog() {
    shellDialogs.closeSubmitReview()
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
      submitError = coreErrorMessage(result.error)
      return
    }
    hydrateFromSnapshot(await fetchSnapshot())
    closeDialog()
  }

  $effect(() => {
    if (open) {
      dismissOpenPopover()
      reset()
    }
  })
</script>

<Dialog.Root {open} onOpenChange={(next) => !next && closeDialog()}>
  <Dialog.Content class="max-w-md gap-0 p-0">
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
      </Button>
      <Button size="xl" onclick={() => void submit()} disabled={submitting}>
        {#if submitting}
          <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
          Posting…
        {:else}
          Submit review
        {/if}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
