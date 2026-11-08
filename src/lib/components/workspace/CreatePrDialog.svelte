<script lang="ts">
  import { refreshSnapshot } from '$lib/core/sync'
  import { tick, untrack } from 'svelte'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import * as Dialog from '$lib/components/ui/dialog/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import { Input } from '$lib/components/ui/input/index.js'
  import { Textarea } from '$lib/components/ui/textarea/index.js'
  import DialogShortcut from '$lib/components/shell/DialogShortcut.svelte'
  import { commands } from '$lib/ipc'
  import { formatPrRouteLine } from '$lib/create-pr/route-line'
  import { coreErrorText } from '$lib/core/core-error'
  import { onModEnter } from '$lib/keyboard/mod-enter'
  import { SKELETON_WIDTHS } from '$lib/primitives/skeleton'
  import { shellDialogs, workspaces } from '$lib/state'
  import { stacks } from '$lib/changes/stacks.svelte'
  import { workspaceDiff } from '$lib/changes/diff-store.svelte'

  const workspaceId = $derived(
    shellDialogs.args('createPr')?.workspaceId ?? null,
  )
  const open = $derived(workspaceId != null)
  const workspace = $derived(
    workspaceId ? workspaces.getById(workspaceId) : undefined,
  )

  let why = $state('')
  let title = $state('')
  let drafting = $state(false)
  let creating = $state(false)
  let whyError = $state(false)
  let submitError = $state<string | null>(null)
  let hint = $state('')
  let draftGeneration = $state(0)

  let whyField = $state<HTMLTextAreaElement | null>(null)

  const diffFiles = $derived(
    workspaceId ? (workspaceDiff.filesByWorkspace[workspaceId] ?? []) : [],
  )
  const diffTotals = $derived(workspaceDiff.totals(workspaceId ?? ''))
  // A stacked branch's PR targets the nearest unmerged branch below it, and the
  // counts are against that branch, matching what the backend opens.
  const stackBranch = $derived.by(() => {
    const level = workspace && stacks.branchFor(workspace.id, workspace.branch)
    return level && !level.merged ? level : undefined
  })
  const routeLine = $derived(
    workspace
      ? formatPrRouteLine(
          stackBranch
            ? {
                branch: workspace.branch,
                base: stackBranch.parent,
                fileCount: stackBranch.files,
                totals: {
                  added: stackBranch.additions,
                  deleted: stackBranch.deletions,
                },
              }
            : {
                branch: workspace.branch,
                base: workspace.base,
                fileCount: diffFiles.length,
                totals: diffTotals,
              },
        )
      : '',
  )

  const MIN_DRAFT_MS = 1200

  async function startDraft() {
    if (!workspaceId || !workspace) return
    const generation = ++draftGeneration
    drafting = true
    whyError = false
    submitError = null
    why = ''
    hint = 'Drafting from the thread and diff…'

    const minDelay = new Promise((resolve) => setTimeout(resolve, MIN_DRAFT_MS))
    const draftPromise = commands.draftPrWhy(workspaceId)

    const [draftResult] = await Promise.all([draftPromise, minDelay])
    if (generation !== draftGeneration || !open) return

    if (draftResult.status === 'error') {
      drafting = false
      submitError = coreErrorText(draftResult.error)
      hint = 'Edit the description before creating.'
      return
    }

    why = draftResult.data.text
    drafting = false
    hint = 'Drafted from the thread and diff. Edit it before creating.'
    await tick()
    whyField?.focus()
  }

  // Each opening starts over with a fresh draft.
  $effect(() => {
    const id = workspaceId
    if (!id) return
    untrack(() => {
      title = workspaces.getById(id)?.name ?? ''
      creating = false
      submitError = null
      whyError = false
      void tick().then(startDraft)
    })
  })

  function closeDialog() {
    draftGeneration += 1
    shellDialogs.close('createPr')
    drafting = false
    creating = false
  }

  function onWhyInput() {
    if (why.trim()) whyError = false
  }

  async function submit() {
    if (!workspaceId || creating || drafting) return
    const trimmed = why.trim()
    if (!trimmed) {
      whyError = true
      whyField?.focus()
      return
    }
    creating = true
    submitError = null
    const result = await commands.createWorkspacePullRequest({
      workspaceId,
      body: trimmed,
      title: title.trim() || null,
    })
    creating = false
    if (result.status === 'error') {
      submitError = coreErrorText(result.error)
      return
    }
    closeDialog()
    await refreshSnapshot()
  }

  const onFormKeydown = onModEnter(() => void submit())
</script>

<Dialog.Root {open} onOpenChange={(next) => !next && closeDialog()}>
  <Dialog.Content
    id="pr-dialog"
    data-od-id="pr-dialog"
    class="max-w-[min(880px,calc(100vw-4rem))] sm:max-w-[min(880px,calc(100vw-4rem))] gap-0 p-0"
  >
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <form
      class="flex flex-col"
      id="pr-form"
      novalidate
      onsubmit={(e) => {
        e.preventDefault()
        void submit()
      }}
      onkeydown={onFormKeydown}
    >
      <Dialog.Header class="px-5 pt-5 gap-1">
        <Dialog.Title>Create draft pull request</Dialog.Title>
        {#if routeLine}
          <p
            class="text-sm text-muted-foreground font-mono"
            data-od-id="pr-route"
          >
            {routeLine}
          </p>
        {/if}
      </Dialog.Header>

      <div class="flex flex-col gap-5 p-5">
        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="pr-title">Title</label>
          <Input
            id="pr-title"
            bind:value={title}
            disabled={creating || drafting}
          />
        </div>

        <div class="grid gap-1.5" data-od-id="pr-why-field">
          <div class="flex items-center justify-between gap-2">
            <label class="text-sm font-medium" for="pr-why">
              Description
            </label>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              data-od-id="pr-redraft"
              disabled={drafting || creating}
              onclick={() => void startDraft()}
            >
              Redraft
            </Button>
          </div>
          <div class="relative">
            <Textarea
              bind:ref={whyField}
              id="pr-why"
              class="h-[min(560px,calc(100vh-22rem))] min-h-40 resize-none field-sizing-fixed px-4 py-3 leading-relaxed"
              placeholder="Why this change is necessary, what changed, and how it was tested"
              bind:value={why}
              readonly={drafting}
              aria-busy={drafting}
              aria-invalid={whyError}
              aria-describedby="pr-why-hint pr-why-error"
              disabled={creating}
              oninput={onWhyInput}
            />
            {#if drafting}
              <div
                class="pointer-events-none absolute inset-px grid content-start gap-3 rounded-lg bg-popover px-5 py-4"
                aria-hidden="true"
              >
                {#each SKELETON_WIDTHS as width, index (index)}
                  <span
                    class="h-2.5 rounded bg-linear-to-r from-muted via-muted-foreground/20 to-muted bg-size-[200%_100%] animate-[shimmer_1.4s_linear_infinite]"
                    class:mt-3={index === 3}
                    style:width
                  ></span>
                {/each}
              </div>
            {/if}
          </div>
          <p
            id="pr-why-hint"
            class="text-sm text-muted-foreground"
            aria-live="polite"
            hidden={whyError}
          >
            {hint}
          </p>
          {#if whyError}
            <p
              id="pr-why-error"
              class="text-sm text-destructive flex items-center gap-1.5"
            >
              Add a description so reviewers know what this changes.
            </p>
          {/if}
        </div>

        {#if submitError}
          <p class="text-sm text-destructive" role="alert">{submitError}</p>
        {/if}
      </div>

      <Dialog.Footer class="m-0 px-5 py-4 flex-row items-center justify-end">
        <div class="flex gap-2 ml-auto">
          <Button
            size="xl"
            type="button"
            variant="ghost"
            disabled={creating}
            onclick={closeDialog}
          >
            Cancel
            <DialogShortcut keys="cancel" />
          </Button>
          <Button
            size="xl"
            type="submit"
            data-od-id="pr-submit"
            disabled={drafting || creating}
          >
            {#if creating}
              <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
              Creating PR…
            {:else}
              Create PR
              <DialogShortcut keys="submit" />
            {/if}
          </Button>
        </div>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<style>
  @keyframes shimmer {
    0% {
      background-position: 200% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }
</style>
