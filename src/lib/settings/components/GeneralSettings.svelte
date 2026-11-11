<script lang="ts">
  import { reloadEnvironment, updateSettings } from '$lib/settings/actions'
  import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import * as Select from '$lib/components/ui/select'
  import { Switch } from '$lib/components/ui/switch'
  import SettingsPanel from '$lib/settings/components/SettingsPanel.svelte'
  import SettingsRow from '$lib/settings/components/SettingsRow.svelte'
  import { resolveDefaultRepoId } from '$lib/settings/default-repo'
  import { repos } from '$lib/state'
  import { settings } from '$lib/settings/store.svelte'
  import { workspaces } from '$lib/state/workspaces.svelte'
  import { Alert, AlertDescription } from '$lib/components/ui/alert'

  let reloadingEnv = $state(false)
  // Follows the saved setting, and holds edits until they're committed.
  let worktreeDraft = $derived(settings.values.worktreeRoot)
  let terminalDraft = $derived(settings.values.terminalApp)
  let editorDraft = $derived(settings.values.editorApp)

  const defaultRepoId = $derived(
    resolveDefaultRepoId(repos.items, settings.values.defaultRepo),
  )
  const defaultRepoName = $derived(
    repos.items.find((repo) => repo.id === defaultRepoId)?.name ?? '',
  )

  const hasWorkspaces = $derived(workspaces.items.length > 0)

  async function reload() {
    reloadingEnv = true
    await reloadEnvironment()
    reloadingEnv = false
  }

  async function commitWorktreeRoot() {
    if (worktreeDraft.trim() === settings.values.worktreeRoot.trim()) return
    await updateSettings({ worktreeRoot: worktreeDraft })
  }
</script>

<SettingsPanel data-od-id="settings-general-group">
  <SettingsRow
    title="Default repository"
    description="Pre-selected whenever you pick a repo for a new thread"
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
            if (value) void updateSettings({ defaultRepo: value })
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
        {#if hasWorkspaces && worktreeDraft.trim() !== settings.values.worktreeRoot.trim()}
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
          if (terminalDraft.trim() !== settings.values.terminalApp)
            void updateSettings({ terminalApp: terminalDraft })
        }}
      />
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Editor app"
    description="What Open in Editor launches, by its name in Applications (e.g. Zed, Cursor, Visual Studio Code)"
    controlId="settings-editor-app"
  >
    {#snippet control()}
      <Input
        id="settings-editor-app"
        class="w-[180px]"
        placeholder="Zed"
        bind:value={editorDraft}
        spellcheck={false}
        onchange={() => {
          if (editorDraft.trim() !== settings.values.editorApp)
            void updateSettings({ editorApp: editorDraft })
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
        bind:checked={
          () => settings.values.reduceMotion,
          (next) => void updateSettings({ reduceMotion: next })
        }
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
        onclick={() => void reload()}
      >
        {#if reloadingEnv}
          <LoaderCircleIcon class="size-4 animate-spin" aria-hidden="true" />
        {/if}
        Reload environment
      </Button>
    {/snippet}
  </SettingsRow>
</SettingsPanel>
