<script lang="ts">
  import { onDestroy } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import { Textarea } from '$lib/components/ui/textarea'

  // One editable prompt in Settings › Prompts. Saves as you type; matching the
  // default saves nothing, so a later change to the default still applies.
  let {
    id,
    label,
    hint,
    saved,
    defaultPrompt,
    onSave,
  }: {
    id: string
    label: string
    hint: string
    /** What's stored; empty means the default. */
    saved: string
    defaultPrompt: string
    onSave: (next: string) => void
  } = $props()

  /** Edits live here so a snapshot refresh mid-typing can't overwrite them. */
  let draft = $state<string | null>(null)

  const value = $derived(draft ?? (saved || defaultPrompt))
  const customised = $derived(
    value.trim() !== '' && value.trim() !== defaultPrompt.trim(),
  )

  let saveTimer: ReturnType<typeof setTimeout> | undefined

  function save() {
    saveTimer = undefined
    if (draft == null) return
    const trimmed = draft.trim()
    onSave(trimmed === defaultPrompt.trim() ? '' : trimmed)
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
    onSave('')
  }

  onDestroy(() => {
    if (saveTimer) {
      clearTimeout(saveTimer)
      save()
    }
  })
</script>

<div
  class="glass-card space-y-1.5 rounded-2xl px-4 py-3"
  data-od-id="settings-{id}"
>
  <div class="flex items-center justify-between gap-2">
    <label class="text-sm font-medium" for="settings-{id}-field">
      {label}
    </label>
    {#if customised}
      <Button type="button" variant="ghost" size="sm" onclick={reset}>
        Reset to default
      </Button>
    {/if}
  </div>
  <p class="text-xs text-muted-foreground" id="settings-{id}-hint">
    {hint}
  </p>
  <Textarea
    id="settings-{id}-field"
    class="min-h-24"
    {value}
    oninput={onInput}
    aria-describedby="settings-{id}-hint"
  />
</div>
