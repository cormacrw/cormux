<script lang="ts">
  import { onMount, tick } from 'svelte'
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle'
  import * as Dialog from '$lib/components/ui/dialog/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import { Input } from '$lib/components/ui/input/index.js'
  import * as Select from '$lib/components/ui/select/index.js'
  import { Textarea } from '$lib/components/ui/textarea/index.js'
  import DialogShortcut from '$lib/components/shell/DialogShortcut.svelte'
  import { commands } from '$lib/ipc'
  import type { EngineKind, EngineStatus } from '$lib/ipc/bindings'
  import { fetchSnapshot } from '$lib/ipc'
  import { coreErrorText } from '$lib/feedback/core-error'
  import { draftBranchName, fallbackBranchName } from '$lib/new-workspace/draft'
  import {
    ENGINE_OPTIONS,
    engineHint,
    engineInstallLabel,
  } from '$lib/new-workspace/engines'
  import {
    branchErrorMessage,
    validateBaseBranch,
    validateBranchName,
  } from '$lib/new-workspace/validation'
  import { dismissOpenPopover } from '$lib/keyboard/global-shortcuts'
  import { resolveDefaultRepoId } from '$lib/new-workspace/settings-defaults'
  import {
    app,
    homebaseUi,
    hydrateFromSnapshot,
    repos,
    settings,
    shellDialogs,
    workspaceRecords,
    workspaces,
  } from '$lib/state'
  import BaseBranchCombobox from './BaseBranchCombobox.svelte'

  let open = $state(false)
  let submitting = $state(false)
  let submitError = $state<string | null>(null)

  let branchName = $state('')
  let repoId = $state('')
  let baseBranch = $state('')
  let engine = $state<EngineKind>('claude')
  let prompt = $state('')

  let branchEdited = $state(false)
  let branchError = $state<string | null>(null)
  let baseError = $state<string | null>(null)

  let repoBranches = $state<string[]>([])
  let engineStatuses = $state<EngineStatus[]>([])

  let promptInput = $state<HTMLTextAreaElement | null>(null)
  let baseCombo: BaseBranchCombobox | undefined = $state()

  const workspaceBranchesOnRepo = $derived(
    workspaceRecords.records
      .filter((row) => row.repoId === repoId)
      .map((row) => row.branch),
  )

  const branchList = $derived.by(() => {
    const extra = workspaceBranchesOnRepo.filter(
      (branch) => !repoBranches.includes(branch),
    )
    return [...repoBranches, ...extra]
  })

  const engineStatusByKind = $derived(
    new Map(engineStatuses.map((row) => [row.kind, row])),
  )

  const engineHintText = $derived(engineHint())

  async function loadBranches(forRepoId: string) {
    if (!forRepoId) {
      repoBranches = []
      return
    }
    const result = await commands.listRepoBranches(forRepoId)
    if (result.status === 'ok') {
      repoBranches = result.data.branches
    }
  }

  async function loadEngines() {
    const result = await commands.detectEngines()
    if (result.status === 'ok') engineStatuses = result.data
  }

  function repoDefaultBranch(forRepoId: string) {
    return repos.getById(forRepoId)?.defaultBranch?.trim() || 'main'
  }

  function resetForm() {
    branchName = ''
    prompt = ''
    engine = settings.defaultEngine
    repoId = resolveDefaultRepoId(repos.items, settings.defaultRepo)
    baseBranch = repoDefaultBranch(repoId)
    branchEdited = false
    branchError = null
    baseError = null
    submitError = null
    submitting = false
  }

  async function prepareOpen() {
    dismissOpenPopover()
    window.dispatchEvent(new CustomEvent('cormux:close-palette'))
    if (open) return
    resetForm()
    open = true
    shellDialogs.newWorkspaceOpen = true
    await loadEngines()
    await loadBranches(repoId)
    await tick()
    promptInput?.focus()
  }

  function closeDialog() {
    open = false
    shellDialogs.newWorkspaceOpen = false
    app.newWorkspaceRequested = false
  }

  function onPromptInput() {
    if (branchEdited) return
    branchName = prompt.trim() ? draftBranchName(prompt) : ''
    branchError = null
  }

  function onBranchInput() {
    branchEdited = branchName.trim().length > 0
    if (!branchName.trim()) branchEdited = false
    if (branchError) branchError = null
  }

  function engineLabel(kind: EngineKind) {
    const option = ENGINE_OPTIONS.find((row) => row.kind === kind)
    const install = engineInstallLabel(engineStatusByKind.get(kind))
    return `${option?.label ?? kind}${install ? ` — ${install}` : ''}`
  }

  function onRepoChange(next: string) {
    if (!next || next === repoId) return
    repoId = next
    baseBranch = repoDefaultBranch(repoId)
    baseError = null
    void loadBranches(repoId)
  }

  function branchValidationContext() {
    return {
      existingBranches: repoBranches,
      defaultBranch: repoDefaultBranch(repoId),
      workspaceBranches: workspaceRecords.records.map((row) => row.branch),
    }
  }

  function branchCode(value: string) {
    const code = validateBranchName(value, branchValidationContext())
    const pr = app.newWorkspacePullRequest
    // Continuing a PR checks out its existing branch instead of creating one.
    if (
      code === 'exists' &&
      pr?.mode === 'continue' &&
      value.trim() === pr.branch
    ) {
      return null
    }
    return code
  }

  function takenBranches() {
    return [
      ...repoBranches,
      ...workspaceRecords.records.map((row) => row.branch),
    ]
  }

  function resolvedBranch() {
    return (
      branchName.trim() ||
      draftBranchName(prompt) ||
      fallbackBranchName(takenBranches())
    )
  }

  function validateAll(): boolean {
    const branchValue = resolvedBranch()
    const code = branchCode(branchValue)
    if (code) {
      branchError = branchErrorMessage(branchValue, code)
      return false
    }
    const baseMsg = validateBaseBranch(baseBranch, branchList)
    if (baseMsg) {
      baseError = baseMsg
      return false
    }
    return true
  }

  async function submit() {
    if (submitting) return
    submitError = null
    if (!validateAll()) {
      if (branchError) document.getElementById('nw-branch')?.focus()
      else if (baseError) document.getElementById('nw-base')?.focus()
      return
    }

    const branch = resolvedBranch()
    submitting = true

    const result = await commands.createWorkspace({
      repoId,
      // The core numbers it: Workspace 1, Workspace 2, …
      name: '',
      branch,
      base: baseBranch.trim(),
      engine,
      goal: prompt.trim(),
    })

    if (result.status === 'error') {
      submitting = false
      submitError = coreErrorText(result.error, 'Could not create workspace')
      return
    }

    homebaseUi.resetFilter()
    closeDialog()
    hydrateFromSnapshot(await fetchSnapshot())
    app.openWorkspace(result.data.workspaceId)
  }

  function onDialogKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && baseCombo?.isOpen()) {
      event.preventDefault()
      baseCombo.closeList()
    }
  }

  function onFormKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault()
      void submit()
    }
  }

  onMount(() => {
    const onRequest = () => {
      void prepareOpen()
    }
    window.addEventListener('cormux:new-workspace', onRequest)
    return () => window.removeEventListener('cormux:new-workspace', onRequest)
  })

  $effect(() => {
    if (app.newWorkspaceRequested && !open) void prepareOpen()
  })

  $effect(() => {
    const pr = app.newWorkspacePullRequest
    if (!open || !pr) return
    if (pr.mode === 'review') return
    repoId =
      pr.repoId ?? resolveDefaultRepoId(repos.items, settings.defaultRepo)
    branchName = pr.branch
    branchEdited = true
    prompt = pr.title
    void loadBranches(repoId)
  })
</script>

<Dialog.Root bind:open onOpenChange={(next) => !next && closeDialog()}>
  <Dialog.Content
    class="max-w-[720px] sm:max-w-[720px] gap-0 p-0"
    onkeydown={onDialogKeydown}
  >
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <form
      class="flex flex-col"
      novalidate
      onsubmit={(e) => {
        e.preventDefault()
        void submit()
      }}
      onkeydown={onFormKeydown}
    >
      <Dialog.Header class="px-5 pt-5">
        <Dialog.Title>New workspace</Dialog.Title>
      </Dialog.Header>

      <div class="flex flex-col gap-5 p-5">
        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="nw-prompt"
            >Initial prompt</label
          >
          <Textarea
            bind:ref={promptInput}
            id="nw-prompt"
            rows={4}
            class="min-h-[112px] max-h-[280px] resize-y"
            placeholder="e.g., Implement OAuth login with Supabase"
            bind:value={prompt}
            oninput={onPromptInput}
            aria-describedby="nw-prompt-hint"
          />
          <p id="nw-prompt-hint" class="text-xs text-muted-foreground">
            Optional. Leave blank and the thread waits until you send a message.
          </p>
        </div>

        <div class="grid gap-4 min-[760px]:grid-cols-2">
          <div class="grid gap-1.5">
            <label class="text-sm font-medium" for="nw-engine">AI engine</label>
            <Select.Root
              type="single"
              value={engine}
              onValueChange={(next) => {
                if (next) engine = next as EngineKind
              }}
            >
              <Select.Trigger
                id="nw-engine"
                class="w-full"
                aria-describedby="nw-engine-hint"
              >
                {engineLabel(engine)}
              </Select.Trigger>
              <Select.Content>
                {#each ENGINE_OPTIONS as option (option.kind)}
                  <Select.Item value={option.kind} label={engineLabel(option.kind)}>
                    {engineLabel(option.kind)}
                  </Select.Item>
                {/each}
              </Select.Content>
            </Select.Root>
            <p id="nw-engine-hint" class="text-xs text-muted-foreground">
              {engineHintText}
            </p>
          </div>
          <div class="grid gap-1.5">
            <label class="text-sm font-medium" for="nw-repo">Repository</label>
            {#if repos.items.length === 0}
              <div
                id="nw-repo"
                class="sunken text-muted-foreground flex h-10 items-center px-3.5 text-sm"
              >
                No repositories yet
              </div>
              <p id="nw-repo-hint" class="text-xs text-muted-foreground">
                Add a repo in Settings before creating a workspace.
              </p>
            {:else}
              <Select.Root
                type="single"
                value={repoId}
                onValueChange={onRepoChange}
              >
                <Select.Trigger
                  id="nw-repo"
                  class="w-full"
                  aria-describedby="nw-repo-hint"
                >
                  {repos.items.find((repo) => repo.id === repoId)?.name}
                </Select.Trigger>
                <Select.Content>
                  {#each repos.items as repo (repo.id)}
                    <Select.Item value={repo.id} label={repo.name}
                      >{repo.name}</Select.Item
                    >
                  {/each}
                </Select.Content>
              </Select.Root>
              <p id="nw-repo-hint" class="text-xs text-muted-foreground">
                The worktree is created from this repo
              </p>
            {/if}
          </div>
        </div>

        <div class="grid gap-4 min-[760px]:grid-cols-2">
          <div class="grid gap-1.5">
            <label class="text-sm font-medium" for="nw-base">Base branch</label>
            <BaseBranchCombobox
              bind:this={baseCombo}
              id="nw-base"
              hintId="nw-base-hint"
              errorId="nw-base-error"
              bind:value={baseBranch}
              branches={branchList}
              workspaceBranches={workspaces.items.map((row) => row.branch)}
              bind:error={baseError}
            />
            {#if baseError}
              <p id="nw-base-error" class="text-xs text-destructive">
                {baseError}
              </p>
            {:else}
              <p id="nw-base-hint" class="text-xs text-muted-foreground">
                Your new branch starts from here
              </p>
            {/if}
          </div>
          <div class="grid gap-1.5">
            <label class="text-sm font-medium" for="nw-branch"
              >Branch name</label
            >
            <Input
              id="nw-branch"
              class="font-mono"
              spellcheck={false}
              placeholder="feat/oauth-login"
              bind:value={branchName}
              oninput={onBranchInput}
              onblur={() => {
                const value = branchName.trim()
                if (!value) return
                const code = branchCode(value)
                branchError = code ? branchErrorMessage(value, code) : null
              }}
              aria-invalid={branchError ? 'true' : undefined}
              aria-describedby="nw-branch-hint nw-branch-error"
            />
            <p id="nw-branch-hint" class="text-xs text-muted-foreground">
              New branch created for this worktree
            </p>
            {#if branchError}
              <p id="nw-branch-error" class="text-xs text-destructive">
                {branchError}
              </p>
            {:else}
              <span id="nw-branch-error" class="sr-only"></span>
            {/if}
          </div>
        </div>

        {#if submitError}
          <p class="text-sm text-destructive">{submitError}</p>
        {/if}
      </div>

      <Dialog.Footer class="items-center m-0 px-5 py-4">
        <Button
          size="xl"
          type="button"
          variant="ghost"
          disabled={submitting}
          onclick={closeDialog}>Cancel <DialogShortcut keys="cancel" /></Button
        >
        <Button size="xl" type="submit" disabled={submitting || !repoId}>
          {#if submitting}
            <LoaderCircleIcon class="size-4 animate-spin" aria-hidden="true" />
            Creating worktree…
          {:else}
            Create Workspace
            <DialogShortcut keys="submit" />
          {/if}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
