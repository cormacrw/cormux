<script lang="ts">
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import * as Select from '$lib/components/ui/select'
  import { Switch } from '$lib/components/ui/switch'
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import { commands } from '$lib/ipc'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { resolveDefaultRepoId } from '$lib/new-workspace/settings-defaults'
  import { repos, settings, workspaceRecords } from '$lib/state'
  import { Alert, AlertDescription } from '$lib/components/ui/alert'

  let reloadingEnv = $state(false)
  // Follows the saved setting, and holds edits until they're committed.
  let worktreeDraft = $derived(settings.worktreeRoot)
  let terminalDraft = $derived(settings.terminalApp)

  const defaultRepoId = $derived(
    resolveDefaultRepoId(repos.items, settings.defaultRepo),
  )
  const defaultRepoName = $derived(
    repos.items.find((repo) => repo.id === defaultRepoId)?.name ?? '',
  )

  const hasWorkspaces = $derived(workspaceRecords.records.length > 0)

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
    title="Default repository"
    description="Pre-selected whenever you pick a repo for a new workspace or scratch"
    controlId="settings-default-repo"
  >
    {#snippet control()}
      {#if repos.items.length === 0}
        <span class="text-xs text-muted-foreground">Add a repo first</span>
      {:else}
        <Select.Root
          type="single"
          value={defaultRepoId}
          onValueChange={(value) => {
            if (value) void settings.setDefaultRepo(value)
          }}
        >
          <Select.Trigger
            id="settings-default-repo"
            class="w-[180px]"
            data-od-id="settings-default-repo"
          >
            <span class="truncate">{defaultRepoName}</span>
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
    title="Terminal app"
    description="What Open in Terminal launches, by its name in Applications (e.g. iTerm, Ghostty, Warp)"
    controlId="settings-terminal-app"
  >
    {#snippet control()}
      <Input
        id="settings-terminal-app"
        class="w-[180px]"
        placeholder="Terminal"
        bind:value={terminalDraft}
        spellcheck={false}
        onchange={() => {
          if (terminalDraft.trim() !== settings.terminalApp)
            void settings.setTerminalApp(terminalDraft)
        }}
      />
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
