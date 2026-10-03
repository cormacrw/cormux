<script lang="ts">
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import * as Select from '$lib/components/ui/select'
  import {
    CLICKUP_FOLDER_KEY,
    CLICKUP_SPACE_KEY,
    CLICKUP_WORKSPACE_KEY,
  } from '$lib/clickup/board'
  import { coreErrorText } from '$lib/feedback/core-error'
  import { commands } from '$lib/ipc'
  import type { ClickupOption, Error as CoreError } from '$lib/ipc/bindings'
  import { readStringSetting } from '$lib/new-workspace/settings-defaults'
  import { clickup, settings } from '$lib/state'

  let keyDraft = $state('')
  let saving = $state(false)
  let keyError = $state<string | null>(null)

  let workspaceOptions = $state<ClickupOption[]>([])
  let spaceOptions = $state<ClickupOption[]>([])
  let folderOptions = $state<ClickupOption[]>([])
  let loadError = $state<string | null>(null)

  const workspaceId = $derived(
    readStringSetting(settings.rows, CLICKUP_WORKSPACE_KEY, ''),
  )
  const spaceId = $derived(
    readStringSetting(settings.rows, CLICKUP_SPACE_KEY, ''),
  )
  const folderId = $derived(
    readStringSetting(settings.rows, CLICKUP_FOLDER_KEY, ''),
  )

  async function saveKey() {
    const key = keyDraft.trim()
    if (!key || saving) return
    saving = true
    keyError = null
    const result = await commands.setClickupApiKey(key)
    saving = false
    if (result.status === 'error') {
      keyError = coreErrorText(result.error, 'Could not save the key')
      return
    }
    keyDraft = ''
  }

  async function removeKey() {
    const result = await commands.clearClickupApiKey()
    if (result.status === 'error') {
      keyError = coreErrorText(result.error, 'Could not remove the key')
    }
  }

  /** Picking a parent clears its children, so the board never points at another space's folder. */
  async function choose(level: 'workspace' | 'space' | 'folder', id: string) {
    if (level === 'workspace') {
      await settings.persist(CLICKUP_WORKSPACE_KEY, id)
      await settings.persist(CLICKUP_SPACE_KEY, '')
      await settings.persist(CLICKUP_FOLDER_KEY, '')
    } else if (level === 'space') {
      await settings.persist(CLICKUP_SPACE_KEY, id)
      await settings.persist(CLICKUP_FOLDER_KEY, '')
    } else {
      await settings.persist(CLICKUP_FOLDER_KEY, id)
    }
  }

  function unwrap(
    result:
      | { status: 'ok'; data: ClickupOption[] }
      | { status: 'error'; error: CoreError },
  ) {
    if (result.status === 'ok') return result.data
    loadError = coreErrorText(result.error, 'Could not reach ClickUp')
    return []
  }

  /** With one choice there's nothing to ask, so it's picked for the user. */
  function autoPick(
    options: ClickupOption[],
    current: string,
    level: 'workspace' | 'space' | 'folder',
    prefer?: (option: ClickupOption) => boolean,
  ) {
    if (current || options.length === 0) return
    const preferred = prefer ? options.filter(prefer) : []
    const only =
      options.length === 1
        ? options[0]
        : preferred.length === 1
          ? preferred[0]
          : null
    if (only) void choose(level, only.id)
  }

  $effect(() => {
    if (!clickup.configured) return
    loadError = null
    void commands.clickupWorkspaces().then((result) => {
      workspaceOptions = unwrap(result)
    })
  })

  $effect(() => {
    spaceOptions = []
    if (!clickup.configured || !workspaceId) return
    const id = workspaceId
    void commands.clickupSpaces(id).then((result) => {
      if (id === workspaceId) spaceOptions = unwrap(result)
    })
  })

  $effect(() => {
    folderOptions = []
    if (!clickup.configured || !spaceId) return
    const id = spaceId
    void commands.clickupFolders(id).then((result) => {
      if (id === spaceId) folderOptions = unwrap(result)
    })
  })

  // Separate from the fetches so a snapshot that briefly rolls a pick back gets it re-made.
  $effect(() => autoPick(workspaceOptions, workspaceId, 'workspace'))
  $effect(() => autoPick(spaceOptions, spaceId, 'space'))
  $effect(() =>
    autoPick(folderOptions, folderId, 'folder', (option) =>
      /sprint/i.test(option.name),
    ),
  )

  function nameOf(options: ClickupOption[], id: string, placeholder: string) {
    return options.find((option) => option.id === id)?.name ?? placeholder
  }
</script>

{#snippet picker(
  level: 'workspace' | 'space' | 'folder',
  options: ClickupOption[],
  value: string,
  placeholder: string,
)}
  <Select.Root
    type="single"
    {value}
    onValueChange={(next) => {
      if (next && next !== value) void choose(level, next)
    }}
  >
    <Select.Trigger
      id="settings-clickup-{level}"
      class="w-[220px]"
      disabled={options.length === 0}
      data-od-id="settings-clickup-{level}"
    >
      <span class="truncate">{nameOf(options, value, placeholder)}</span>
    </Select.Trigger>
    <Select.Content>
      {#each options as option (option.id)}
        <Select.Item value={option.id} label={option.name}
          >{option.name}</Select.Item
        >
      {/each}
    </Select.Content>
  </Select.Root>
{/snippet}

<SettingsPanel data-od-id="settings-clickup">
  <SettingsRow
    title="API key"
    controlId={clickup.configured ? undefined : 'settings-clickup-key'}
    description={clickup.configured
      ? 'Stored in ~/.cormux/credentials.json. Remove it to hide the Sprint page.'
      : 'A personal token from ClickUp › Settings › Apps (starts with pk_). Saved to ~/.cormux/credentials.json.'}
  >
    {#snippet control()}
      {#if clickup.configured}
        <div class="flex items-center gap-3">
          <span class="text-xs font-medium">Connected</span>
          <Button variant="outline" size="sm" onclick={removeKey}>Remove</Button
          >
        </div>
      {:else}
        <form
          class="flex items-center gap-2"
          onsubmit={(event) => {
            event.preventDefault()
            void saveKey()
          }}
        >
          <Input
            id="settings-clickup-key"
            type="password"
            autocomplete="off"
            spellcheck={false}
            placeholder="pk_…"
            class="w-[220px] font-mono"
            bind:value={keyDraft}
            aria-invalid={keyError ? 'true' : undefined}
            aria-describedby={keyError
              ? 'settings-clickup-key-error'
              : undefined}
          />
          <Button
            type="submit"
            variant="secondary"
            size="sm"
            disabled={!keyDraft.trim() || saving}
          >
            {saving ? 'Checking…' : 'Save'}
          </Button>
        </form>
      {/if}
    {/snippet}
  </SettingsRow>

  {#if clickup.configured}
    <SettingsRow
      title="Workspace"
      controlId="settings-clickup-workspace"
      description="The ClickUp workspace your team plans in."
    >
      {#snippet control()}
        {@render picker('workspace', workspaceOptions, workspaceId, 'Choose…')}
      {/snippet}
    </SettingsRow>
    <SettingsRow
      title="Space"
      controlId="settings-clickup-space"
      description="The space that holds your sprints."
    >
      {#snippet control()}
        {@render picker('space', spaceOptions, spaceId, 'Choose…')}
      {/snippet}
    </SettingsRow>
    <SettingsRow
      title="Sprint folder"
      controlId="settings-clickup-folder"
      description="The folder whose lists are your sprints. The board shows the one whose dates include today."
    >
      {#snippet control()}
        {@render picker('folder', folderOptions, folderId, 'Choose…')}
      {/snippet}
    </SettingsRow>
  {/if}
</SettingsPanel>

{#if keyError || loadError}
  <p
    id="settings-clickup-key-error"
    class="mt-2 text-xs text-destructive"
    role="alert"
  >
    {keyError ?? loadError}
  </p>
{/if}
