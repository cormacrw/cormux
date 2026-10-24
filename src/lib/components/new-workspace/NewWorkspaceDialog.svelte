<script lang="ts">
  import Bot from '@lucide/svelte/icons/bot'
  import Folder from '@lucide/svelte/icons/folder'
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
    engineInstallLabel,
    soleInstalledEngine,
  } from '$lib/new-workspace/engines'
  import {
    branchErrorMessage,
    validateBaseBranch,
    validateBranchName,
    workspaceBranchesForRepo,
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

  const soleEngine = $derived(soleInstalledEngine(engineStatuses))

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
    await loadEngines()
    const only = soleInstalledEngine(engineStatuses)
    if (only) engine = only
    open = true
    shellDialogs.newWorkspaceOpen = true
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

  async function onRepoChange(value: string) {
    if (!value || value === repoId) return
    repoId = value
    baseBranch = repoDefaultBranch(repoId)
    baseError = null
    await loadBranches(repoId)
  }

  const repoWorkspaceBranches = $derived(
    workspaceBranchesForRepo(workspaceRecords.records, repoId),
  )

  function branchValidationContext() {
    return {
      existingBranches: repoBranches,
      defaultBranch: repoDefaultBranch(repoId),
      workspaceBranches: repoWorkspaceBranches,
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
    return [...repoBranches, ...repoWorkspaceBranches]
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
    class="max-w-[780px] sm:max-w-[780px] gap-0 p-0"
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
      <Dialog.Header class="px-5 pt-4">
        <Dialog.Title class="text-sm font-semibold text-muted-foreground"
          >New workspace</Dialog.Title
        >
      </Dialog.Header>

      <div class="flex flex-col gap-4 px-5 pt-3 pb-5">
        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="nw-prompt"
            >Initial prompt</label
          >
          <Textarea
            bind:ref={promptInput}
            id="nw-prompt"
            rows={4}
            class="max-h-[280px] min-h-[112px] resize-y text-[15px] leading-relaxed"
            placeholder="e.g., Implement OAuth login with Supabase"
            bind:value={prompt}
            oninput={onPromptInput}
            aria-describedby="nw-prompt-hint"
          />
          <p id="nw-prompt-hint" class="text-xs text-muted-foreground">
            Optional. Leave blank and the thread waits until you send a message.
          </p>
        </div>

        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="nw-branch">Branch name</label>
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
          {#if branchError}
            <p id="nw-branch-error" class="text-xs text-destructive">
              {branchError}
            </p>
          {:else}
            <p id="nw-branch-hint" class="text-xs text-muted-foreground">
              Drafted from the prompt · a new branch for this worktree
            </p>
            <span id="nw-branch-error" class="sr-only"></span>
          {/if}
        </div>

        {#if baseError}
          <p id="nw-base-error" class="text-xs text-destructive">
            {baseError}
          </p>
        {/if}
        {#if submitError}
          <p class="text-sm text-destructive">{submitError}</p>
        {/if}
      </div>

      <!-- Where the worktree comes from sits with the actions, as compact pickers. -->
      <div
        class="flex flex-wrap items-center gap-1.5 border-t border-border/60 px-4 py-3"
      >
        {#if repos.items.length === 0}
          <span
            id="nw-repo"
            class="glass-well flex h-8 items-center rounded-[9px] px-2.5 text-xs text-muted-foreground"
            title="Add a repo in Settings before creating a workspace."
          >
            No repositories yet
          </span>
        {:else}
          <label class="sr-only" for="nw-repo">Repository</label>
          <Select.Root
            type="single"
            value={repoId}
            onValueChange={(value) => void onRepoChange(value)}
          >
            <Select.Trigger
              id="nw-repo"
              size="sm"
              class="glass-well h-8 max-w-[168px] gap-1.5 rounded-[9px] border-transparent text-xs"
              title="The worktree is created from this repo"
            >
              <Folder class="size-3.5 shrink-0 opacity-70" aria-hidden="true" />
              <span class="truncate"
                >{repos.items.find((repo) => repo.id === repoId)?.name ??
                  'Pick a repository'}</span
              >
            </Select.Trigger>
            <Select.Content>
              {#each repos.items as repo (repo.id)}
                <Select.Item value={repo.id} label={repo.name}
                  >{repo.name}</Select.Item
                >
              {/each}
            </Select.Content>
          </Select.Root>
        {/if}

        <div class="w-[176px]" title="Your new branch starts from here">
          <label class="sr-only" for="nw-base">Base branch</label>
          <BaseBranchCombobox
            bind:this={baseCombo}
            id="nw-base"
            hintId="nw-base-hint"
            errorId="nw-base-error"
            bind:value={baseBranch}
            branches={branchList}
            workspaceBranches={repoWorkspaceBranches}
            bind:error={baseError}
          />
          <span id="nw-base-hint" class="sr-only"
            >Your new branch starts from here</span
          >
        </div>

        {#if !soleEngine}
          <label class="sr-only" for="nw-engine">AI engine</label>
          <Select.Root
            type="single"
            value={engine}
            onValueChange={(value) => {
              if (value) engine = value as EngineKind
            }}
          >
            <Select.Trigger
              id="nw-engine"
              size="sm"
              class="glass-well h-8 gap-1.5 rounded-[9px] border-transparent text-xs"
              title="The agent provider to use"
            >
              <Bot class="size-3.5 shrink-0 opacity-70" aria-hidden="true" />
              <span class="truncate"
                >{ENGINE_OPTIONS.find((option) => option.kind === engine)
                  ?.label ?? engine}</span
              >
            </Select.Trigger>
            <Select.Content>
              {#each ENGINE_OPTIONS as option (option.kind)}
                {@const install = engineInstallLabel(
                  engineStatusByKind.get(option.kind),
                )}
                <Select.Item value={option.kind} label={option.label}>
                  {option.label}
                  {#if install}
                    <span class="ml-auto text-xs text-muted-foreground"
                      >{install}</span
                    >
                  {/if}
                </Select.Item>
              {/each}
            </Select.Content>
          </Select.Root>
        {/if}

        <div class="ml-auto flex shrink-0 items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            disabled={submitting}
            onclick={closeDialog}
            >Cancel <DialogShortcut keys="cancel" /></Button
          >
          <Button type="submit" disabled={submitting || !repoId}>
            {#if submitting}
              <LoaderCircleIcon
                class="size-4 animate-spin"
                aria-hidden="true"
              />
              Creating worktree…
            {:else}
              Create Workspace
              <DialogShortcut keys="submit" />
            {/if}
          </Button>
        </div>
      </div>
    </form>
  </Dialog.Content>
</Dialog.Root>
