<script lang="ts">
  import { updateSettings } from '$lib/settings/actions'
  import { tick } from 'svelte'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import Plus from '@lucide/svelte/icons/plus'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Textarea } from '$lib/components/ui/textarea'
  import { resolveDefaultRepoId } from '$lib/settings/default-repo'
  import { bangName, newBangId, type Bang } from '$lib/bangs/bangs'
  import { commands } from '$lib/ipc'
  import { coreErrorText } from '$lib/core/core-error'
  import { repos } from '$lib/state'
  import { settings } from '$lib/settings/store.svelte'
  import { cn } from '$lib/utils'
  import { debouncedSaves } from '$lib/primitives/debounced-saves'

  let nameDraft = $state('')
  let nameError = $state<string | null>(null)
  /** Why the core refused the last edit to an open bang; its name rules are the core's. */
  let editError = $state<string | null>(null)

  /** Edits live here so a snapshot refresh mid-typing can't overwrite them. */
  let drafts = $state<Record<string, { name: string; prompt: string }>>({})

  const defaultRepoId = $derived(
    resolveDefaultRepoId(repos.items, settings.values.defaultRepo),
  )

  const saves = debouncedSaves()

  function isExpanded(id: string) {
    return id in drafts
  }

  function expand(bang: Bang) {
    drafts[bang.id] = { name: bang.name, prompt: bang.prompt }
  }

  function collapse(id: string) {
    flushSave()
    delete drafts[id]
  }

  function withDrafts(): Bang[] {
    return settings.values.bangs.map((bang) => {
      const draft = drafts[bang.id]
      return draft ? { ...bang, name: draft.name, prompt: draft.prompt } : bang
    })
  }

  /** Saves the list as the core takes it, which tidies the names; why it refused, if it did. */
  async function saveBangs(bangs: Bang[]): Promise<string | null> {
    const result = await commands.updateSettings({ bangs })
    if (result.status === 'error') {
      return coreErrorText(result.error, 'Could not save the bangs')
    }
    settings.hydrate(result.data)
    return null
  }

  function flushSave() {
    saves.flush('bangs')
  }

  function scheduleSave() {
    saves.schedule('bangs', async () => {
      editError = await saveBangs(withDrafts())
    })
  }

  function toggleConfigure(bang: Bang) {
    if (isExpanded(bang.id)) {
      collapse(bang.id)
      return
    }
    expand(bang)
    void tick().then(() => {
      document.getElementById(`bang-prompt-${bang.id}`)?.focus()
    })
  }

  async function removeBang(id: string) {
    flushSave()
    delete drafts[id]
    await updateSettings({
      bangs: settings.values.bangs.filter((bang) => bang.id !== id),
    })
    await tick()
    document.getElementById('settings-bang-name')?.focus()
  }

  async function submitAdd(event: Event) {
    event.preventDefault()
    flushSave()
    const id = newBangId()
    const error = await saveBangs([
      ...settings.values.bangs,
      { id, name: nameDraft, prompt: '' },
    ])
    if (error) {
      nameError = error
      document.getElementById('settings-bang-name')?.focus()
      return
    }
    const bang = settings.values.bangs.find((saved) => saved.id === id)
    nameDraft = ''
    nameError = null
    if (!bang) return
    expand(bang)
    await tick()
    document.getElementById(`bang-prompt-${bang.id}`)?.focus()
  }
</script>

{#if settings.values.bangs.length > 0}
  <ul
    class="set-group mt-4 space-y-2"
    aria-labelledby="settings-bangs-h"
    data-od-id="settings-bang-list"
  >
    {#each settings.values.bangs as bang (bang.id)}
      {@const panelOpen = isExpanded(bang.id)}
      <li
        class="glass-card overflow-hidden rounded-2xl"
        data-od-id="settings-bang-{bang.id}"
      >
        <div class="flex flex-wrap items-center gap-2 px-4 py-3">
          <span class="min-w-0 flex-1">
            <span class="block font-mono text-sm font-medium">!{bang.name}</span
            >
            <span
              class="block truncate text-xs text-muted-foreground"
              title={bang.prompt}
            >
              {bang.prompt.trim() || 'No prompt'}
            </span>
          </span>
          {#if !bang.prompt.trim()}
            <span class="text-xs font-medium text-amber-500">
              Needs a prompt
            </span>
          {/if}
          <Button
            type="button"
            variant="ghost"
            size="sm"
            class="gap-1"
            aria-expanded={panelOpen}
            aria-controls="bang-cfg-{bang.id}"
            aria-label="Configure !{bang.name}"
            onclick={() => toggleConfigure(bang)}
          >
            Configure
            <ChevronDown
              class={cn(
                'size-3 transition-transform',
                panelOpen && 'rotate-180',
              )}
            />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            class="gap-1"
            aria-label="Remove !{bang.name}"
            onclick={() => void removeBang(bang.id)}
          >
            <Trash2 class="size-3.5" aria-hidden="true" />
            Remove
          </Button>
        </div>
        {#if panelOpen && drafts[bang.id]}
          <div
            class="space-y-4 border-t border-border px-4 py-4"
            id="bang-cfg-{bang.id}"
          >
            <div class="field space-y-1.5">
              <label class="text-sm font-medium" for="bang-name-{bang.id}">
                Name
              </label>
              <Input
                id="bang-name-{bang.id}"
                class="font-mono"
                maxlength={32}
                autocomplete="off"
                spellcheck={false}
                bind:value={drafts[bang.id]!.name}
                oninput={scheduleSave}
                aria-describedby="bang-name-hint-{bang.id}"
              />
              <p
                class="field-hint text-xs text-muted-foreground"
                id="bang-name-hint-{bang.id}"
              >
                No spaces. Type !{bangName(drafts[bang.id]!.name) || bang.name} in
                a prompt bar to put the prompt in, or in the command palette to start
                a thread with it.
              </p>
            </div>
            <div class="field space-y-1.5">
              <label class="text-sm font-medium" for="bang-prompt-{bang.id}">
                Prompt
              </label>
              <Textarea
                id="bang-prompt-{bang.id}"
                rows={4}
                placeholder="Run the tests, then fix whatever fails."
                bind:value={drafts[bang.id]!.prompt}
                oninput={scheduleSave}
                aria-describedby="bang-prompt-hint-{bang.id}"
              />
              <p
                class="field-hint text-xs text-muted-foreground"
                id="bang-prompt-hint-{bang.id}"
              >
                From the command palette, the thread starts in
                {defaultRepoId || 'your default repository'} and runs in the background;
                words typed after the bang are added to the end.
              </p>
            </div>
          </div>
        {/if}
      </li>
    {/each}
  </ul>
  {#if editError}
    <p class="mt-2 text-xs text-destructive" role="alert">{editError}</p>
  {/if}
{/if}

<form
  class="glass-card mt-4 grid gap-2 rounded-2xl p-4"
  data-od-id="settings-bang-add"
  novalidate
  onsubmit={(event) => void submitAdd(event)}
>
  <label class="text-sm font-medium" for="settings-bang-name">Add a bang</label>
  <div class="flex flex-wrap gap-2">
    <!-- The field fills the row, as tall as the button beside it, so the ! centres in it. -->
    <div class="relative min-w-[240px] flex-1">
      <span
        class="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 font-mono text-sm text-muted-foreground"
        aria-hidden="true">!</span
      >
      <Input
        id="settings-bang-name"
        bind:value={nameDraft}
        placeholder="test"
        maxlength={32}
        autocomplete="off"
        spellcheck={false}
        class="h-full pl-6 font-mono"
        aria-invalid={nameError ? true : undefined}
        aria-describedby={nameError
          ? 'settings-bang-error'
          : 'settings-bang-hint'}
        oninput={() => {
          if (nameError) nameError = null
        }}
      />
    </div>
    <Button type="submit" variant="secondary" size="lg" class="gap-1">
      <Plus class="size-4" aria-hidden="true" />
      Add bang
    </Button>
  </div>
  {#if nameError}
    <p class="text-xs text-destructive" id="settings-bang-error" role="alert">
      {nameError}
    </p>
  {:else}
    <p class="text-xs text-muted-foreground" id="settings-bang-hint">
      A short name with no spaces. You'll write the prompt next.
    </p>
  {/if}
</form>
