<script lang="ts">
  import { onMount, tick } from 'svelte'
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle'
  import * as Dialog from '$lib/components/ui/dialog/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import { Input } from '$lib/components/ui/input/index.js'
  import { Textarea } from '$lib/components/ui/textarea/index.js'
  import { Kbd, KbdGroup } from '$lib/components/ui/kbd/index.js'
  import { commands } from '$lib/ipc'
  import type { EngineKind, EngineStatus } from '$lib/ipc/bindings'
  import { fetchSnapshot } from '$lib/ipc'
  import { draftBranchName, draftWorkspaceName } from '$lib/new-workspace/draft'
  import {
    ENGINE_OPTIONS,
    engineHint,
    engineInstallLabel,
  } from '$lib/new-workspace/engines'
  import {
    branchErrorMessage,
    validateBaseBranch,
    validateBranchName,
    validatePrompt,
  } from '$lib/new-workspace/validation'
  import { dismissOpenPopover } from '$lib/keyboard/global-shortcuts'
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

  let workspaceName = $state('')
  let branchName = $state('')
  let repoId = $state('')
  let baseBranch = $state('')
  let engine = $state<EngineKind>('claude')
  let prompt = $state('')

  let nameEdited = $state(false)
  let branchEdited = $state(false)
  let promptError = $state(false)
  let branchError = $state<string | null>(null)
  let baseError = $state<string | null>(null)

  let repoBranches = $state<string[]>([])
  let engineStatuses = $state<EngineStatus[]>([])

  let nameInput = $state<HTMLInputElement | null>(null)
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

  const engineHintText = $derived(engineHint(engine))

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

  function defaultRepoId() {
    if (repos.items.some((repo) => repo.id === 'my-app')) return 'my-app'
    return repos.items[0]?.id ?? ''
  }

  function resetForm() {
    workspaceName = ''
    branchName = ''
    prompt = ''
    engine = settings.defaultEngine
    repoId = defaultRepoId()
    baseBranch = settings.defaultBase
    nameEdited = false
    branchEdited = false
    promptError = false
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
    nameInput?.focus()
  }

  function closeDialog() {
    open = false
    shellDialogs.newWorkspaceOpen = false
    app.newWorkspaceRequested = false
  }

  function onPromptInput() {
    const trimmed = prompt.trim()
    if (!nameEdited) workspaceName = trimmed ? draftWorkspaceName(prompt) : ''
    if (!branchEdited) {
      branchName = trimmed ? draftBranchName(prompt) : ''
      branchError = null
    }
    if (!trimmed) {
      if (!nameEdited) workspaceName = ''
      if (!branchEdited) branchName = ''
    }
    if (trimmed) promptError = false
  }

  function onNameInput() {
    nameEdited = workspaceName.trim().length > 0
    if (!workspaceName.trim()) nameEdited = false
  }

  function onBranchInput() {
    branchEdited = branchName.trim().length > 0
    if (!branchName.trim()) branchEdited = false
    if (branchError) branchError = null
  }

  async function onRepoChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement
    repoId = select.value
    const repo = repos.getById(repoId)
    baseBranch = repo?.defaultBranch?.trim() || settings.defaultBase || 'main'
    baseError = null
    await loadBranches(repoId)
  }

  function branchValidationContext() {
    return {
      existingBranches: repoBranches,
      workspaceBranches: workspaceRecords.records.map((row) => row.branch),
    }
  }

  function validateAll(): boolean {
    if (!validatePrompt(prompt)) {
      promptError = true
      return false
    }
    const branchValue = branchName.trim() || draftBranchName(prompt)
    const branchCode = validateBranchName(
      branchValue,
      branchValidationContext(),
    )
    if (branchCode) {
      branchError = branchErrorMessage(branchValue, branchCode)
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
      if (promptError) document.getElementById('nw-prompt')?.focus()
      else if (branchError) document.getElementById('nw-branch')?.focus()
      else if (baseError) document.getElementById('nw-base')?.focus()
      return
    }

    const name = workspaceName.trim() || draftWorkspaceName(prompt)
    const branch = branchName.trim() || draftBranchName(prompt)
    submitting = true

    const result = await commands.createWorkspace({
      repoId,
      name: name.slice(0, 48),
      branch,
      base: baseBranch.trim(),
      engine,
      goal: prompt.trim(),
    })

    if (result.status === 'error') {
      submitting = false
      submitError = result.error.message ?? 'Could not create workspace'
      return
    }

    homebaseUi.resetFilter()
    app.openHomebase()
    closeDialog()
    hydrateFromSnapshot(await fetchSnapshot())
  }

  function onDialogKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && baseCombo?.isOpen()) {
      event.preventDefault()
      baseCombo.closeList()
    }
  }

  function onFormKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      const target = event.target as HTMLElement
      if (target.tagName === 'TEXTAREA') {
        event.preventDefault()
        void submit()
      }
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
    repoId = pr.repoId ?? defaultRepoId()
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
      <Dialog.Header class="border-b px-4 py-3">
        <Dialog.Title>New workspace</Dialog.Title>
      </Dialog.Header>

      <div class="flex flex-col gap-4 px-4 py-4">
        <div class="grid gap-4 min-[760px]:grid-cols-2">
          <div class="grid gap-1.5">
            <label class="text-sm font-medium" for="nw-name"
              >Workspace name</label
            >
            <Input
              bind:ref={nameInput}
              id="nw-name"
              maxlength={48}
              placeholder="OAuth login"
              bind:value={workspaceName}
              oninput={onNameInput}
              aria-describedby="nw-name-hint"
            />
            <p id="nw-name-hint" class="text-xs text-muted-foreground">
              Shown in the sidebar and on Homebase
            </p>
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
                const code = validateBranchName(
                  value,
                  branchValidationContext(),
                )
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

        <div class="grid gap-4 min-[760px]:grid-cols-2">
          <div class="grid gap-1.5">
            <label class="text-sm font-medium" for="nw-repo">Repository</label>
            {#if repos.items.length === 0}
              <div
                id="nw-repo"
                class="border-input bg-input/30 text-muted-foreground flex h-8 items-center rounded-lg border px-2.5 text-sm"
              >
                No repositories yet
              </div>
              <p id="nw-repo-hint" class="text-xs text-muted-foreground">
                Add a repo in Settings before creating a workspace.
              </p>
            {:else}
              <select
                id="nw-repo"
                class="border-input bg-input/30 h-8 w-full rounded-lg border px-2.5 text-sm text-foreground"
                value={repoId}
                onchange={onRepoChange}
                aria-describedby="nw-repo-hint"
              >
                {#each repos.items as repo (repo.id)}
                  <option value={repo.id}>{repo.name}</option>
                {/each}
              </select>
              <p id="nw-repo-hint" class="text-xs text-muted-foreground">
                The worktree is created from this repo
              </p>
            {/if}
          </div>
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
        </div>

        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="nw-engine">AI engine</label>
          <select
            id="nw-engine"
            class="border-input bg-input/30 h-8 w-full rounded-lg border px-2.5 text-sm text-foreground"
            bind:value={engine}
            aria-describedby="nw-engine-hint"
          >
            {#each ENGINE_OPTIONS as option (option.kind)}
              {@const status = engineStatusByKind.get(option.kind)}
              {@const install = engineInstallLabel(status)}
              <option value={option.kind}>
                {option.label}{install ? ` — ${install}` : ''}
              </option>
            {/each}
          </select>
          <p id="nw-engine-hint" class="text-xs text-muted-foreground">
            {engineHintText}
          </p>
        </div>

        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="nw-prompt"
            >Initial prompt</label
          >
          <Textarea
            id="nw-prompt"
            rows={4}
            class="min-h-[112px] max-h-[280px] resize-y"
            placeholder="e.g., Implement OAuth login with Supabase"
            bind:value={prompt}
            oninput={onPromptInput}
            onblur={() => {
              if (!validatePrompt(prompt)) promptError = true
            }}
            aria-invalid={promptError ? 'true' : undefined}
            aria-describedby="nw-prompt-hint nw-prompt-error"
          />
          {#if promptError}
            <p id="nw-prompt-error" class="text-xs text-destructive">
              Describe the task so the agent knows where to start.
            </p>
          {:else}
            <span id="nw-prompt-error" class="sr-only"></span>
          {/if}
        </div>

        {#if submitError}
          <p class="text-sm text-destructive">{submitError}</p>
        {/if}
      </div>

      <Dialog.Footer class="px-4 py-3 sm:justify-between">
        <p class="text-xs text-muted-foreground">Esc to cancel</p>
        <Button type="submit" disabled={submitting || !repoId}>
          {#if submitting}
            <LoaderCircleIcon class="size-4 animate-spin" aria-hidden="true" />
            Creating worktree…
          {:else}
            Create Workspace
            <KbdGroup class="ml-2 hidden sm:inline-flex">
              <Kbd>⌘</Kbd>
              <Kbd>↵</Kbd>
            </KbdGroup>
          {/if}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
