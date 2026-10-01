<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { Textarea } from '$lib/components/ui/textarea'
  import { commands } from '$lib/ipc'
  import { settings } from '$lib/state'

  let defaultPrompt = $state('')
  /** Edits live here so a snapshot refresh mid-typing can't overwrite them. */
  let draft = $state<string | null>(null)

  const value = $derived(draft ?? (settings.prPrompt || defaultPrompt))
  const customised = $derived(
    value.trim() !== '' && value.trim() !== defaultPrompt.trim(),
  )

  let saveTimer: ReturnType<typeof setTimeout> | undefined

  function save() {
    saveTimer = undefined
    if (draft == null) return
    // Matching the default stores nothing, so a later default change still applies.
    const trimmed = draft.trim()
    void settings.setPrPrompt(trimmed === defaultPrompt.trim() ? '' : trimmed)
  }

  function onInput(event: Event) {
    draft = (event.currentTarget as HTMLTextAreaElement).value
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(save, 350)
  }

  function reset() {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = undefined
    draft = null
    void settings.setPrPrompt('')
  }

  onMount(async () => {
    defaultPrompt = await commands.defaultPrPrompt()
  })

  onDestroy(() => {
    if (saveTimer) {
      clearTimeout(saveTimer)
      save()
    }
  })
</script>

<div
  class="mt-4 space-y-1.5 rounded-lg border border-border bg-card/40 px-4 py-3"
  data-od-id="settings-pr-prompt"
>
  <div class="flex items-center justify-between gap-2">
    <label class="text-sm font-medium" for="settings-pr-prompt-field">
      PR prompt
    </label>
    {#if customised}
      <Button type="button" variant="ghost" size="sm" onclick={reset}>
        Reset to default
      </Button>
    {/if}
  </div>
  <p class="text-xs text-muted-foreground" id="settings-pr-prompt-hint">
    What the agent is asked when Create PR drafts a description. The thread,
    goal and changed files are added after it.
  </p>
  <Textarea
    id="settings-pr-prompt-field"
    class="min-h-40"
    {value}
    oninput={onInput}
    aria-describedby="settings-pr-prompt-hint"
  />
</div>
