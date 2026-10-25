<script lang="ts">
  import { tick } from 'svelte'
  import { open } from '@tauri-apps/plugin-dialog'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import Folder from '@lucide/svelte/icons/folder'
  import Plus from '@lucide/svelte/icons/plus'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Input } from '$lib/components/ui/input'
  import { Textarea } from '$lib/components/ui/textarea'
  import RepoDefaultBranchField from '$lib/components/settings/RepoDefaultBranchField.svelte'
  import { commands } from '$lib/ipc'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { showToast } from '$lib/feedback/show-toast'
  import {
    addRepoErrorMessage,
    removeRepoBlockReason,
    repoDomKey,
    validateAddRepoPath,
    workspaceCountForRepo,
  } from '$lib/settings/repos-validation'
  import { repos, settings, workspaceRecords } from '$lib/state'
  import { cn } from '$lib/utils'

  let pathDraft = $state('')
  let pathError = $state<string | null>(null)
  let adding = $state(false)
  let testingRepoId = $state<string | null>(null)

  let expandedIds = $state<string[]>([])

  function isExpanded(repoId: string) {
    return expandedIds.includes(repoId)
  }

  function setExpanded(repoId: string, open: boolean) {
    if (open) {
      if (!expandedIds.includes(repoId)) {
        expandedIds = [...expandedIds, repoId]
      }
      return
    }
    expandedIds = expandedIds.filter((id) => id !== repoId)
  }

  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- timer handles, never rendered
  const saveTimers = new Map<string, ReturnType<typeof setTimeout>>()

  function debouncedSave(repoId: string, fn: () => void) {
    const existing = saveTimers.get(repoId)
    if (existing) clearTimeout(existing)
    saveTimers.set(
      repoId,
      setTimeout(() => {
        saveTimers.delete(repoId)
        fn()
      }, 350),
    )
  }

  function effectiveRun(repo: { runCommand: string | null }) {
    return (repo.runCommand ?? '').trim()
  }

  $effect(() => {
    const repoId = settings.expandRepoId
    if (!repoId) return
    setExpanded(repoId, true)
    void tick().then(() => {
      settings.expandRepoId = null
    })
  })

  $effect(() => {
    const repoId = settings.focusRepoSetup
    if (!repoId) return
    setExpanded(repoId, true)
    void tick().then(async () => {
      await scrollRepoIntoView(repoId)
      document.getElementById(`setup-${repoId}`)?.focus()
      settings.focusRepoSetup = null
    })
  })

  $effect(() => {
    const repoId = settings.focusRepoRunCommand
    if (!repoId) return
    setExpanded(repoId, true)
    void tick().then(async () => {
      await scrollRepoIntoView(repoId)
      document.getElementById(`run-${repoId}`)?.focus()
      settings.focusRepoRunCommand = null
    })
  })

  async function scrollRepoIntoView(repoId: string) {
    const key = repoDomKey(repoId)
    const el = document.querySelector(`[data-od-id="settings-repo-${key}"]`)
    el?.scrollIntoView({
      behavior: settings.reduceMotion ? 'auto' : 'smooth',
      block: 'start',
    })
  }

  function coreErrorMessage(error: unknown) {
    if (typeof error === 'object' && error && 'message' in error) {
      return String((error as { message: string }).message)
    }
    return String(error)
  }

  function patchRepo(
    repoId: string,
    patch: Partial<(typeof repos.items)[number]>,
  ) {
    repos.hydrate(
      repos.items.map((repo) =>
        repo.id === repoId ? { ...repo, ...patch } : repo,
      ),
    )
  }

  async function persistRunCommand(repoId: string, value: string) {
    const trimmed = value.trim()
    const result = await commands.setRepoRunCommand({
      repoId,
      runCommand: trimmed || null,
    })
    if (result.status === 'error') {
      toastCoreError(result.error)
    }
  }

  async function persistSetupCommands(repoId: string, value: string) {
    const result = await commands.setRepoSetupCommands({
      repoId,
      setupCommands: value,
    })
    if (result.status === 'error') {
      toastCoreError(result.error)
    }
  }

  async function saveSingleInstance(repoId: string, singleInstance: boolean) {
    patchRepo(repoId, { singleInstance })
    const result = await commands.setRepoSingleInstance({
      repoId,
      singleInstance,
    })
    if (result.status === 'error') {
      patchRepo(repoId, { singleInstance: !singleInstance })
      toastCoreError(result.error)
    }
  }

  /**
   * What's being typed in a field (`run-<id>` or `setup-<id>`). The field shows
   * this, not the saved value, until you leave it: saves land a beat later and
   * the refresh they cause carries the value as saved (trimmed, and missing
   * whatever was typed since), which would jump the text and caret under you.
   */
  let drafts = $state<Record<string, string>>({})

  function fieldValue(key: string, saved: string) {
    return drafts[key] ?? saved
  }

  function saveRunCommand(repoId: string, value: string) {
    drafts[`run-${repoId}`] = value
    debouncedSave(repoId, () => void persistRunCommand(repoId, value))
  }

  function saveSetupCommands(repoId: string, value: string) {
    drafts[`setup-${repoId}`] = value
    debouncedSave(
      `setup-${repoId}`,
      () => void persistSetupCommands(repoId, value),
    )
  }

  /** Leaving a field saves now and shows the saved value again. */
  async function finishField(timerKey: string, draftKey: string) {
    const pending = saveTimers.get(timerKey)
    const value = drafts[draftKey]
    if (pending) {
      clearTimeout(pending)
      saveTimers.delete(timerKey)
    }
    if (value === undefined) return
    const repoId = draftKey.replace(/^(run|setup)-/, '')
    if (pending) {
      await (draftKey.startsWith('run-')
        ? persistRunCommand(repoId, value)
        : persistSetupCommands(repoId, value))
    }
    patchRepo(
      repoId,
      draftKey.startsWith('run-')
        ? { runCommand: value.trim() || null }
        : { setupCommands: value },
    )
    delete drafts[draftKey]
  }

  async function pickFolder() {
    const selected = await open({ directory: true, multiple: false })
    if (typeof selected === 'string') {
      pathDraft = selected
      pathError = null
    }
  }

  function validatePathClient() {
    const code = validateAddRepoPath(pathDraft, repos.items)
    if (!code) {
      pathError = null
      return true
    }
    const segment = pathDraft.trim().replace(/\/+$/, '').split('/').pop()
    pathError = addRepoErrorMessage(code, segment)
    return false
  }

  async function submitAdd(event: Event) {
    event.preventDefault()
    if (!validatePathClient()) {
      document.getElementById('settings-repo-path')?.focus()
      return
    }
    adding = true
    const normalized = pathDraft.trim().replace(/\/+$/, '')
    const result = await commands.addRepo({ path: normalized })
    adding = false
    if (result.status === 'error') {
      pathError = coreErrorMessage(result.error)
      document.getElementById('settings-repo-path')?.focus()
      return
    }
    repos.hydrate([...repos.items, result.data])
    pathDraft = ''
    pathError = null
    settings.expandRepoId = result.data.id
    settings.focusRepoSetup = result.data.id
  }

  async function removeRepo(repoId: string) {
    const result = await commands.removeRepo({ repoId })
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    repos.hydrate(repos.items.filter((repo) => repo.id !== repoId))
    await tick()
    document.getElementById('settings-repo-path')?.focus()
  }

  async function testSetup(repoId: string) {
    testingRepoId = repoId
    const result = await commands.testRepoSetup({ repoId })
    testingRepoId = null
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    showToast({
      tone: result.data.ok ? 'ok' : 'bad',
      parts: [{ type: 'text', value: result.data.message }],
    })
  }

  function toggleConfigure(repoId: string) {
    const next = !isExpanded(repoId)
    setExpanded(repoId, next)
    if (next) {
      void tick().then(() => {
        document.getElementById(`setup-${repoId}`)?.focus()
      })
    }
  }
</script>

<ul
  class="set-group repo-list mt-4 space-y-2"
  aria-labelledby="settings-repos-h"
  data-od-id="settings-repo-list"
>
  {#each repos.items as repo (repo.id)}
    {@const key = repoDomKey(repo.id)}
    {@const used = workspaceCountForRepo(repo.id, workspaceRecords.records)}
    {@const block = removeRepoBlockReason(repo, repos.items.length, used)}
    <!-- Follows what's being typed, so the flag clears as you type, not when the save lands. -->
    {@const run = effectiveRun({
      runCommand: fieldValue(`run-${repo.id}`, repo.runCommand ?? ''),
    })}
    {@const panelOpen = isExpanded(repo.id)}
    <li
      class="repo-item glass-card overflow-hidden rounded-2xl"
      data-od-id="settings-repo-{key}"
    >
      <div class="set-row flex flex-wrap items-center gap-2 px-4 py-3">
        <Folder
          class="size-4 shrink-0 text-muted-foreground"
          aria-hidden="true"
        />
        <span class="txt min-w-0 flex-1">
          <span class="block text-sm font-medium">{repo.name}</span>
          <span
            class="block truncate font-mono text-xs text-muted-foreground"
            title={repo.path}
          >
            {repo.path}
          </span>
        </span>
        {#if used > 0}
          <span class="repo-use text-xs text-muted-foreground">
            {used} workspace{used === 1 ? '' : 's'}
          </span>
        {/if}
        {#if !run}
          <span
            class="repo-flag text-xs font-medium text-amber-500"
            id="repo-flag-{key}"
          >
            No run command
          </span>
        {/if}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          class="repo-cfg-btn gap-1"
          aria-expanded={panelOpen}
          aria-controls="repo-cfg-{key}"
          aria-label="Configure {repo.name}"
          data-od-id="settings-repo-{key}-configure"
          onclick={() => toggleConfigure(repo.id)}
        >
          Configure
          <ChevronDown
            class={cn('size-3 transition-transform', panelOpen && 'rotate-180')}
          />
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          class="gap-1"
          aria-label="Remove {repo.name}"
          disabled={Boolean(block)}
          title={block ?? undefined}
          onclick={() => void removeRepo(repo.id)}
        >
          <Trash2 class="size-3.5" aria-hidden="true" />
          Remove
        </Button>
      </div>
      {#if panelOpen}
        <div
          class="repo-config space-y-4 border-t border-border px-4 py-4"
          id="repo-cfg-{key}"
          data-od-id="settings-repo-{key}-config"
        >
          <RepoDefaultBranchField
            {repo}
            hintId="repo-default-branch-hint-{key}"
          />
          <div class="field space-y-1.5">
            <label class="text-sm font-medium" for="setup-{repo.id}">
              Worktree setup
            </label>
            <Textarea
              id="setup-{repo.id}"
              rows={4}
              spellcheck={false}
              autocomplete="off"
              class="font-mono text-xs"
              placeholder="pnpm install&#10;cp {repo.path}/.env .env"
              aria-describedby="repo-setup-hint-{key}"
              value={fieldValue(`setup-${repo.id}`, repo.setupCommands)}
              oninput={(event) =>
                saveSetupCommands(repo.id, event.currentTarget.value)}
              onblur={() =>
                void finishField(`setup-${repo.id}`, `setup-${repo.id}`)}
            />
            <p
              class="field-hint text-xs text-muted-foreground"
              id="repo-setup-hint-{key}"
            >
              Runs top to bottom in each new worktree before the agent starts.
              One command per line. Use $HARNESS_REPO_PATH for the main
              checkout.
            </p>
          </div>
          <div class="field space-y-1.5">
            <label class="text-sm font-medium" for="run-{repo.id}">
              Run command
            </label>
            <Input
              id="run-{repo.id}"
              value={fieldValue(`run-${repo.id}`, repo.runCommand ?? '')}
              placeholder="pnpm dev"
              spellcheck={false}
              autocomplete="off"
              class="font-mono text-xs"
              aria-describedby="repo-run-hint-{key}"
              oninput={(event) =>
                saveRunCommand(repo.id, event.currentTarget.value)}
              onblur={() => void finishField(repo.id, `run-${repo.id}`)}
            />
            <p
              class="field-hint text-xs text-muted-foreground"
              id="repo-run-hint-{key}"
            >
              Starts the app from the worktree root when you press Run in a
              workspace
            </p>
          </div>
          <div class="field flex items-start gap-2">
            <Checkbox
              id="single-{repo.id}"
              class="mt-0.5"
              checked={repos.items.find((row) => row.id === repo.id)
                ?.singleInstance ?? repo.singleInstance}
              aria-describedby="repo-single-hint-{key}"
              onCheckedChange={(next) => void saveSingleInstance(repo.id, next)}
            />
            <div class="space-y-1.5">
              <label class="text-sm font-medium" for="single-{repo.id}">
                Run one instance at a time
              </label>
              <p
                class="field-hint text-xs text-muted-foreground"
                id="repo-single-hint-{key}"
              >
                Running the app in a workspace stops it in this repo's other
                workspaces
              </p>
            </div>
          </div>
          <div class="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="secondary"
              size="sm"
              disabled={testingRepoId === repo.id}
              onclick={() => void testSetup(repo.id)}
            >
              {testingRepoId === repo.id ? 'Testing…' : 'Test setup'}
            </Button>
          </div>
        </div>
      {/if}
    </li>
  {/each}
</ul>

<form
  class="repo-add glass-card mt-4 grid gap-2 rounded-2xl p-4"
  data-od-id="settings-repo-add"
  novalidate
  onsubmit={(event) => void submitAdd(event)}
>
  <span class="text-sm font-medium">Add a repo</span>
  <div class="repo-add-row flex flex-wrap gap-2">
    <Input
      id="settings-repo-path"
      bind:value={pathDraft}
      placeholder="~/code/my-project"
      spellcheck={false}
      autocomplete="off"
      class="min-w-[240px] flex-1 font-mono text-xs"
      aria-invalid={pathError ? true : undefined}
      aria-describedby={pathError
        ? 'settings-repo-error'
        : 'settings-repo-hint'}
      oninput={() => {
        if (pathError) pathError = null
      }}
    />
    <Button
      type="button"
      variant="outline"
      size="lg"
      onclick={() => void pickFolder()}
    >
      Choose folder…
    </Button>
    <Button
      type="submit"
      variant="secondary"
      size="lg"
      class="gap-1"
      disabled={adding}
      data-od-id="settings-repo-add-btn"
    >
      <Plus class="size-4" aria-hidden="true" />
      Add repo
    </Button>
  </div>
  {#if pathError}
    <p class="text-xs text-destructive" id="settings-repo-error" role="alert">
      {pathError}
    </p>
  {:else}
    <p class="text-xs text-muted-foreground" id="settings-repo-hint">
      The folder that contains the repo's .git directory
    </p>
  {/if}
</form>
