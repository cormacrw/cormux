<script lang="ts">
  import { prefersReducedMotion } from '$lib/motion'
  import { plural } from '$lib/format/plural'
  import { debouncedSaves } from '$lib/primitives/debounced-saves'
  import { tick, untrack } from 'svelte'
  import { open } from '@tauri-apps/plugin-dialog'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import Folder from '@lucide/svelte/icons/folder'
  import Plus from '@lucide/svelte/icons/plus'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import { Button } from '$lib/components/ui/button'
  import { Checkbox } from '$lib/components/ui/checkbox'
  import { Input } from '$lib/components/ui/input'
  import { Textarea } from '$lib/components/ui/textarea'
  import RepoDefaultBranchField from '$lib/settings/components/RepoDefaultBranchField.svelte'
  import { commands } from '$lib/ipc'
  import { coreErrorText } from '$lib/core/core-error'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { showToast } from '$lib/feedback/show-toast'
  import {
    repoDomKey,
    workspaceCountForRepo,
  } from '$lib/settings/repos-validation'
  import { repos, workspaces } from '$lib/state'
  import { nav } from '$lib/core/nav.svelte'
  import { refreshSnapshot } from '$lib/core/sync'
  import type { RepoField } from '$lib/core/route'
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

  const saves = debouncedSaves()

  /** The run command the repo commits in `.cormux/config.json`, used when this field is blank. */
  function committedRun(repoId: string) {
    const run = repos.runCommandById[repoId]
    return run?.source === 'config' ? run.command : ''
  }

  function revealRepo(repoId: string, field?: RepoField) {
    setExpanded(repoId, true)
    if (!field) return
    void tick().then(async () => {
      await scrollRepoIntoView(repoId)
      const id = field === 'setup' ? `setup-${repoId}` : `run-${repoId}`
      document.getElementById(id)?.focus()
    })
  }

  // A link here (the Output panel's empty state) names a repo, and maybe a field of it.
  $effect(() => {
    const route = nav.route
    if (route.view !== 'settings' || !route.repoId) return
    untrack(() => {
      const repoId = nav.take('settings', 'repoId')
      const field = nav.take('settings', 'field')
      if (repoId) revealRepo(repoId, field)
    })
  })

  async function scrollRepoIntoView(repoId: string) {
    const key = repoDomKey(repoId)
    const el = document.querySelector(`[data-od-id="settings-repo-${key}"]`)
    el?.scrollIntoView({
      behavior: prefersReducedMotion() ? 'auto' : 'smooth',
      block: 'start',
    })
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
    const result = await commands.setRepoSingleInstance({
      repoId,
      singleInstance,
    })
    if (result.status === 'error') toastCoreError(result.error)
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
    saves.schedule(repoId, () => void persistRunCommand(repoId, value))
  }

  function saveSetupCommands(repoId: string, value: string) {
    drafts[`setup-${repoId}`] = value
    saves.schedule(
      `setup-${repoId}`,
      () => void persistSetupCommands(repoId, value),
    )
  }

  /** Leaving a field saves now and shows the saved value again. */
  async function finishField(timerKey: string, draftKey: string) {
    const pending = saves.cancel(timerKey)
    const value = drafts[draftKey]
    if (value === undefined) return
    const repoId = draftKey.replace(/^(run|setup)-/, '')
    if (pending) {
      await (draftKey.startsWith('run-')
        ? persistRunCommand(repoId, value)
        : persistSetupCommands(repoId, value))
    }
    await refreshSnapshot()
    delete drafts[draftKey]
  }

  async function pickFolder() {
    const selected = await open({ directory: true, multiple: false })
    if (typeof selected === 'string') {
      pathDraft = selected
      pathError = null
    }
  }

  // The core checks the path (form, folder, repo, duplicates) and says what's wrong.
  async function submitAdd(event: Event) {
    event.preventDefault()
    adding = true
    const result = await commands.addRepo({ path: pathDraft })
    adding = false
    if (result.status === 'error') {
      pathError = coreErrorText(result.error)
      document.getElementById('settings-repo-path')?.focus()
      return
    }
    await refreshSnapshot()
    pathDraft = ''
    pathError = null
    revealRepo(result.data.id, 'setup')
  }

  async function removeRepo(repoId: string) {
    const result = await commands.removeRepo({ repoId })
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
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
    {@const used = workspaceCountForRepo(repo.id, workspaces.items)}
    {@const block = repos.removalBlockById[repo.id] ?? null}
    <!-- Follows what's being typed, so the flag clears as you type, not when the save lands. -->
    {@const run =
      fieldValue(`run-${repo.id}`, repo.runCommand ?? '').trim() ||
      committedRun(repo.id)}
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
            {plural(used, 'workspace')}
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
              placeholder={committedRun(repo.id) || 'pnpm dev'}
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
              checked={repo.singleInstance}
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
