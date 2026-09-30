<script lang="ts">
  import { tick } from 'svelte'
  import ChevronDown from '@lucide/svelte/icons/chevron-down'
  import Plus from '@lucide/svelte/icons/plus'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import Zap from '@lucide/svelte/icons/zap'
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { Textarea } from '$lib/components/ui/textarea'
  import { resolveDefaultRepoId } from '$lib/new-workspace/settings-defaults'
  import {
    newScratchMacroId,
    type ScratchMacro,
  } from '$lib/settings/scratch-macros'
  import { repos, settings } from '$lib/state'
  import { cn } from '$lib/utils'

  let nameDraft = $state('')
  let nameError = $state<string | null>(null)

  /** Edits live here so a snapshot refresh mid-typing can't overwrite them. */
  let drafts = $state<Record<string, { name: string; prompt: string }>>({})

  const defaultRepoId = $derived(
    resolveDefaultRepoId(repos.items, settings.defaultRepo),
  )

  let saveTimer: ReturnType<typeof setTimeout> | undefined

  function isExpanded(id: string) {
    return id in drafts
  }

  function expand(macro: ScratchMacro) {
    drafts[macro.id] = { name: macro.name, prompt: macro.prompt }
  }

  function collapse(id: string) {
    flushSave()
    delete drafts[id]
  }

  function withDrafts(): ScratchMacro[] {
    return settings.scratchMacros.map((macro) => {
      const draft = drafts[macro.id]
      if (!draft) return macro
      // A cleared name keeps the old one rather than leaving the macro unnamed.
      return {
        ...macro,
        name: draft.name.trim() || macro.name,
        prompt: draft.prompt,
      }
    })
  }

  function flushSave() {
    if (!saveTimer) return
    clearTimeout(saveTimer)
    saveTimer = undefined
    void settings.setScratchMacros(withDrafts())
  }

  function scheduleSave() {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(() => {
      saveTimer = undefined
      void settings.setScratchMacros(withDrafts())
    }, 350)
  }

  function toggleConfigure(macro: ScratchMacro) {
    if (isExpanded(macro.id)) {
      collapse(macro.id)
      return
    }
    expand(macro)
    void tick().then(() => {
      document.getElementById(`macro-prompt-${macro.id}`)?.focus()
    })
  }

  async function removeMacro(id: string) {
    flushSave()
    delete drafts[id]
    await settings.setScratchMacros(
      settings.scratchMacros.filter((macro) => macro.id !== id),
    )
    await tick()
    document.getElementById('settings-macro-name')?.focus()
  }

  async function submitAdd(event: Event) {
    event.preventDefault()
    const name = nameDraft.trim()
    if (!name) {
      nameError = 'Give the macro a name.'
      document.getElementById('settings-macro-name')?.focus()
      return
    }
    if (
      settings.scratchMacros.some(
        (macro) => macro.name.trim().toLowerCase() === name.toLowerCase(),
      )
    ) {
      nameError = `A macro named ${name} already exists.`
      document.getElementById('settings-macro-name')?.focus()
      return
    }
    flushSave()
    const macro: ScratchMacro = { id: newScratchMacroId(), name, prompt: '' }
    await settings.setScratchMacros([...settings.scratchMacros, macro])
    nameDraft = ''
    nameError = null
    expand(macro)
    await tick()
    document.getElementById(`macro-prompt-${macro.id}`)?.focus()
  }
</script>

{#if settings.scratchMacros.length > 0}
  <ul
    class="set-group mt-4 space-y-2"
    aria-labelledby="settings-macros-h"
    data-od-id="settings-macro-list"
  >
    {#each settings.scratchMacros as macro (macro.id)}
      {@const panelOpen = isExpanded(macro.id)}
      <li
        class="rounded-lg border border-border"
        data-od-id="settings-macro-{macro.id}"
      >
        <div class="flex flex-wrap items-center gap-2 px-3 py-2">
          <Zap
            class="size-4 shrink-0 text-muted-foreground"
            aria-hidden="true"
          />
          <span class="min-w-0 flex-1">
            <span class="block text-sm font-medium">{macro.name}</span>
            <span
              class="block truncate text-xs text-muted-foreground"
              title={macro.prompt}
            >
              {macro.prompt.trim() || 'No prompt'}
            </span>
          </span>
          {#if !macro.prompt.trim()}
            <span class="text-xs font-medium text-amber-500">
              Not in the palette
            </span>
          {/if}
          <Button
            type="button"
            variant="ghost"
            size="sm"
            class="gap-1"
            aria-expanded={panelOpen}
            aria-controls="macro-cfg-{macro.id}"
            aria-label="Configure {macro.name}"
            onclick={() => toggleConfigure(macro)}
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
            aria-label="Remove {macro.name}"
            onclick={() => void removeMacro(macro.id)}
          >
            <Trash2 class="size-3.5" aria-hidden="true" />
            Remove
          </Button>
        </div>
        {#if panelOpen && drafts[macro.id]}
          <div
            class="space-y-4 border-t border-border px-3 py-3"
            id="macro-cfg-{macro.id}"
          >
            <div class="field space-y-1.5">
              <label class="text-sm font-medium" for="macro-name-{macro.id}">
                Name
              </label>
              <Input
                id="macro-name-{macro.id}"
                maxlength={64}
                autocomplete="off"
                bind:value={drafts[macro.id]!.name}
                oninput={scheduleSave}
                aria-describedby="macro-name-hint-{macro.id}"
              />
              <p
                class="field-hint text-xs text-muted-foreground"
                id="macro-name-hint-{macro.id}"
              >
                Type this in the command palette to run the macro. Also the
                scratch's title.
              </p>
            </div>
            <div class="field space-y-1.5">
              <label class="text-sm font-medium" for="macro-prompt-{macro.id}">
                Prompt
              </label>
              <Textarea
                id="macro-prompt-{macro.id}"
                rows={4}
                placeholder="Summarize what changed on main since yesterday"
                bind:value={drafts[macro.id]!.prompt}
                oninput={scheduleSave}
                aria-describedby="macro-prompt-hint-{macro.id}"
              />
              <p
                class="field-hint text-xs text-muted-foreground"
                id="macro-prompt-hint-{macro.id}"
              >
                Sent as the first message. The scratch starts in
                {defaultRepoId || 'your default repository'} and runs in the background.
              </p>
            </div>
          </div>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<form
  class="mt-4 grid gap-2"
  data-od-id="settings-macro-add"
  novalidate
  onsubmit={(event) => void submitAdd(event)}
>
  <span class="text-sm font-medium">Add a macro</span>
  <div class="flex flex-wrap gap-2">
    <Input
      id="settings-macro-name"
      bind:value={nameDraft}
      placeholder="Morning triage"
      maxlength={64}
      autocomplete="off"
      class="min-w-[240px] flex-1"
      aria-invalid={nameError ? true : undefined}
      aria-describedby={nameError
        ? 'settings-macro-error'
        : 'settings-macro-hint'}
      oninput={() => {
        if (nameError) nameError = null
      }}
    />
    <Button type="submit" variant="secondary" size="lg" class="gap-1">
      <Plus class="size-4" aria-hidden="true" />
      Add macro
    </Button>
  </div>
  {#if nameError}
    <p class="text-xs text-destructive" id="settings-macro-error" role="alert">
      {nameError}
    </p>
  {:else}
    <p class="text-xs text-muted-foreground" id="settings-macro-hint">
      You'll write the prompt next
    </p>
  {/if}
</form>
