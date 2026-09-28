<script lang="ts">
  import { onMount, tick } from 'svelte'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import * as Dialog from '$lib/components/ui/dialog/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import { Input } from '$lib/components/ui/input/index.js'
  import { Textarea } from '$lib/components/ui/textarea/index.js'
  import { Kbd, KbdGroup } from '$lib/components/ui/kbd/index.js'
  import { commands } from '$lib/ipc'
  import type { Error as CoreError } from '$lib/ipc/bindings'
  import { fetchSnapshot } from '$lib/ipc'
  import { formatPrRouteLine } from '$lib/create-pr/route-line'
  import { dismissOpenPopover } from '$lib/keyboard/global-shortcuts'
  import {
    hydrateFromSnapshot,
    shellDialogs,
    workspaceDiff,
    workspaceRecords,
    workspaces,
  } from '$lib/state'

  const workspaceId = $derived(shellDialogs.createPrWorkspaceId)
  const open = $derived(workspaceId != null)
  const workspace = $derived(
    workspaceId ? workspaces.getById(workspaceId) : undefined,
  )
  const record = $derived(
    workspaceId ? workspaceRecords.getRecord(workspaceId) : undefined,
  )

  let why = $state('')
  let title = $state('')
  let draftPr = $state(false)
  let includeWhat = $state(true)
  let includeTested = $state(false)
  let drafting = $state(false)
  let creating = $state(false)
  let whyError = $state(false)
  let submitError = $state<string | null>(null)
  let hint = $state('')
  let draftGeneration = $state(0)

  let whyField: HTMLTextAreaElement | undefined = $state()

  const diffFiles = $derived(
    workspaceId ? (workspaceDiff.filesByWorkspace[workspaceId] ?? []) : [],
  )
  const diffTotals = $derived(workspaceDiff.totals(workspaceId ?? ''))
  const routeLine = $derived(
    record && workspace
      ? formatPrRouteLine({
          branch: workspace.branch,
          base: record.base,
          fileCount: diffFiles.length,
          totals: diffTotals,
        })
      : '',
  )

  const MIN_DRAFT_MS = 1200

  function coreErrorMessage(error: CoreError): string {
    if (
      error.kind === 'Git' ||
      error.kind === 'Workspace' ||
      error.kind === 'Github' ||
      error.kind === 'Llm'
    ) {
      return error.message
    }
    if (error.kind === 'GitConflict') {
      const paths = error.message.paths?.length ?? 0
      return `Git stopped with ${paths} conflicting file${paths === 1 ? '' : 's'}`
    }
    return 'Something went wrong'
  }

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
      submitError = coreErrorMessage(draftResult.error)
      hint = 'Edit the reason before creating.'
      return
    }

    why = draftResult.data.text
    drafting = false
    hint = 'Drafted from the thread and diff. Edit it before creating.'
    await tick()
    whyField?.focus()
  }

  async function prepareOpen(id: string) {
    dismissOpenPopover()
    window.dispatchEvent(new CustomEvent('cormux:close-palette'))
    const ws = workspaces.getById(id)
    if (!ws || ws.prNumber != null || ws.kind === 'review') return
    shellDialogs.openCreatePr(id)
    title = ws.name
    draftPr = false
    includeWhat = true
    includeTested = false
    creating = false
    submitError = null
    whyError = false
    await tick()
    void startDraft()
  }

  function closeDialog() {
    draftGeneration += 1
    shellDialogs.closeCreatePr()
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
      why: trimmed,
      title: title.trim() || null,
      draft: draftPr,
      includeWhatChanged: includeWhat,
      includeHowTested: includeTested,
    })
    creating = false
    if (result.status === 'error') {
      submitError = coreErrorMessage(result.error)
      return
    }
    closeDialog()
    hydrateFromSnapshot(await fetchSnapshot())
  }

  function onFormKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault()
      void submit()
    }
  }

  onMount(() => {
    const onRequest = (event: Event) => {
      const detail = (event as CustomEvent<{ workspaceId: string }>).detail
      if (detail?.workspaceId) void prepareOpen(detail.workspaceId)
    }
    window.addEventListener('cormux:create-pr', onRequest)
    return () => window.removeEventListener('cormux:create-pr', onRequest)
  })
</script>

<Dialog.Root {open} onOpenChange={(next) => !next && closeDialog()}>
  <Dialog.Content
    id="pr-dialog"
    data-od-id="pr-dialog"
    class="max-w-[560px] sm:max-w-[560px] gap-0 p-0"
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
      <Dialog.Header class="border-b px-4 py-3 gap-1">
        <Dialog.Title>Create pull request</Dialog.Title>
        {#if routeLine}
          <p class="text-sm text-muted-foreground font-mono" data-od-id="pr-route">
            {routeLine}
          </p>
        {/if}
      </Dialog.Header>

      <div class="flex flex-col gap-4 px-4 py-4">
        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="pr-title">Title</label>
          <Input id="pr-title" bind:value={title} disabled={creating || drafting} />
        </div>

        <div class="grid gap-1.5" data-od-id="pr-why-field">
          <div class="flex items-center justify-between gap-2">
            <label class="text-sm font-medium" for="pr-why">
              Why is this change necessary?
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
              rows={5}
              placeholder="The problem this solves and why it matters now"
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
                class="pointer-events-none absolute inset-px grid content-start gap-2.5 rounded-lg bg-background p-4"
                aria-hidden="true"
              >
                {#each [1, 2, 3] as line (line)}
                  <span
                    class="h-2 rounded bg-linear-to-r from-muted via-muted-foreground/20 to-muted bg-size-[200%_100%] animate-[shimmer_1.4s_linear_infinite]"
                    class:w-[88%]={line === 2}
                    class:w-[56%]={line === 3}
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
            <p id="pr-why-error" class="text-sm text-destructive flex items-center gap-1.5">
              Add a reason so reviewers know what this fixes.
            </p>
          {/if}
        </div>

        <div class="flex flex-col gap-2 text-sm">
          <label class="flex items-center gap-2">
            <input type="checkbox" bind:checked={includeWhat} disabled={creating} />
            Include what changed in the PR body
          </label>
          <label class="flex items-center gap-2">
            <input type="checkbox" bind:checked={includeTested} disabled={creating} />
            Include how it was tested
          </label>
          <label class="flex items-center gap-2">
            <input type="checkbox" bind:checked={draftPr} disabled={creating} />
            Open as draft PR
          </label>
        </div>

        {#if submitError}
          <p class="text-sm text-destructive" role="alert">{submitError}</p>
        {/if}
      </div>

      <Dialog.Footer class="border-t px-4 py-3 flex-row items-center justify-between">
        <p class="text-xs text-muted-foreground hidden sm:flex items-center gap-1">
          <KbdGroup><Kbd>⌘</Kbd><Kbd>↵</Kbd></KbdGroup>
          to create
        </p>
        <div class="flex gap-2 ml-auto">
          <Button type="button" variant="ghost" disabled={creating} onclick={closeDialog}>
            Cancel
          </Button>
          <Button
            type="submit"
            data-od-id="pr-submit"
            disabled={drafting || creating}
          >
            {#if creating}
              <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
              Creating PR…
            {:else}
              Create PR
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
