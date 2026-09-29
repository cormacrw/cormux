<script lang="ts">
  import { onMount } from 'svelte'
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import * as Select from '$lib/components/ui/select'
  import { Switch } from '$lib/components/ui/switch'
  import * as ToggleGroup from '$lib/components/ui/toggle-group'
  import { setMode, userPrefersMode } from 'mode-watcher'
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import { Kbd, KbdGroup } from '$lib/components/ui/kbd'
  import { commands } from '$lib/ipc'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { SETTINGS_SHORTCUTS } from '$lib/settings/shortcuts'
  import { repos, settings, workspaceRecords } from '$lib/state'
  import { Alert, AlertDescription } from '$lib/components/ui/alert'

  let branches = $state<string[]>([])
  let loadingBranches = $state(false)
  let reloadingEnv = $state(false)
  let worktreeDraft = $state(settings.worktreeRoot)

  const defaultRepoId = $derived(
    repos.items.some((repo) => repo.id === 'my-app')
      ? 'my-app'
      : (repos.items[0]?.id ?? ''),
  )

  const hasWorkspaces = $derived(workspaceRecords.records.length > 0)

  $effect(() => {
    worktreeDraft = settings.worktreeRoot
  })

  onMount(() => {
    void loadBranches()
  })

  async function loadBranches() {
    if (!defaultRepoId) {
      branches = []
      return
    }
    loadingBranches = true
    const result = await commands.listRepoBranches(defaultRepoId)
    loadingBranches = false
    if (result.status === 'ok') {
      branches = result.data.branches
    }
  }

  async function reloadEnvironment() {
    reloadingEnv = true
    const result = await commands.reloadEnvironment()
    reloadingEnv = false
    if (result.status === 'error') toastCoreError(result.error)
  }

  async function commitWorktreeRoot() {
    if (worktreeDraft.trim() === settings.worktreeRoot.trim()) return
    await settings.setWorktreeRoot(worktreeDraft)
  }
</script>

<SettingsPanel data-od-id="settings-general-group">
  <SettingsRow
    title="Default base branch"
    description="New workspaces branch from here unless you pick another"
    controlId="settings-default-base"
  >
    {#snippet control()}
      {#if loadingBranches}
        <LoaderCircleIcon class="size-4 animate-spin text-muted-foreground" />
      {:else if !defaultRepoId}
        <span class="text-xs text-muted-foreground">Add a repo first</span>
      {:else}
        <Select.Root
          type="single"
          value={settings.defaultBase}
          onValueChange={(value) => {
            if (value) void settings.setDefaultBase(value)
          }}
        >
          <Select.Trigger id="settings-default-base" class="w-[180px]">
            <Select.Value placeholder="Branch" />
          </Select.Trigger>
          <Select.Content>
            {#each branches as branch (branch)}
              <Select.Item value={branch}>{branch}</Select.Item>
            {:else}
              <Select.Item value={settings.defaultBase}
                >{settings.defaultBase}</Select.Item
              >
            {/each}
          </Select.Content>
        </Select.Root>
      {/if}
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Teardown after merge"
    description="Delete the worktree once its branch is merged"
  >
    {#snippet control()}
      <Switch
        checked={settings.teardownAfterMerge}
        onCheckedChange={(next) => void settings.setTeardownAfterMerge(next)}
        aria-label="Teardown after merge"
      />
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Worktree location"
    description="Where each workspace's checkout lives on disk. Only new workspaces use a changed path."
    controlId="settings-worktree-root"
  >
    {#snippet control()}
      <div class="flex w-full max-w-md flex-col gap-2 sm:w-auto">
        <Input
          id="settings-worktree-root"
          class="font-mono text-xs"
          bind:value={worktreeDraft}
          spellcheck={false}
          onchange={() => void commitWorktreeRoot()}
        />
        {#if hasWorkspaces && worktreeDraft.trim() !== settings.worktreeRoot.trim()}
          <Alert variant="destructive" class="py-2">
            <AlertDescription class="text-xs">
              Existing workspaces keep their current folders. Changing this only
              affects new checkouts.
            </AlertDescription>
          </Alert>
        {/if}
      </div>
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Appearance"
    description="Follow the system, or always use light or dark"
  >
    {#snippet control()}
      <ToggleGroup.Root
        type="single"
        variant="outline"
        size="sm"
        aria-label="Appearance"
        class="data-[spacing=0]:rounded-lg"
        bind:value={
          () => userPrefersMode.current,
          (next) => {
            // Clicking the pressed item deselects it; keep a mode selected.
            if (next) setMode(next as 'system' | 'light' | 'dark')
          }
        }
      >
        <ToggleGroup.Item value="system">System</ToggleGroup.Item>
        <ToggleGroup.Item value="light">Light</ToggleGroup.Item>
        <ToggleGroup.Item value="dark">Dark</ToggleGroup.Item>
      </ToggleGroup.Root>
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Reduce motion"
    description="Turn off pulses and transitions"
  >
    {#snippet control()}
      <Switch
        checked={settings.reduceMotion}
        onCheckedChange={(next) => void settings.setReduceMotion(next)}
        aria-label="Reduce motion"
      />
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Reload environment"
    description="Re-read your login-shell environment (PATH, API keys, and tool installs)"
  >
    {#snippet control()}
      <Button
        type="button"
        variant="secondary"
        size="sm"
        disabled={reloadingEnv}
        onclick={() => void reloadEnvironment()}
      >
        {#if reloadingEnv}
          <LoaderCircleIcon class="size-4 animate-spin" aria-hidden="true" />
        {/if}
        Reload environment
      </Button>
    {/snippet}
  </SettingsRow>
</SettingsPanel>

<div class="mt-4 space-y-2" data-od-id="settings-shortcuts">
  <h3 class="text-sm font-medium">Keyboard shortcuts</h3>
  <SettingsPanel>
    {#each SETTINGS_SHORTCUTS as row (row.keys)}
      <div
        class="flex items-center justify-between gap-4 border-b border-border px-4 py-2.5 last:border-b-0"
      >
        <span class="text-sm text-muted-foreground">{row.description}</span>
        <KbdGroup>
          {#each row.keys.split('+') as part, index (part + index)}
            {#if index > 0}
              <span class="text-xs text-muted-foreground">+</span>
            {/if}
            <Kbd>{part}</Kbd>
          {/each}
        </KbdGroup>
      </div>
    {/each}
  </SettingsPanel>
</div>
