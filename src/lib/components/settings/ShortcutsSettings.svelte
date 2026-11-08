<script lang="ts">
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import { Kbd, KbdGroup } from '$lib/components/ui/kbd'
  import {
    SHORTCUTS,
    type Shortcut,
    type ShortcutId,
  } from '$lib/keyboard/shortcuts'

  const rows = (Object.keys(SHORTCUTS) as ShortcutId[]).map((id) => {
    const shortcut: Shortcut = SHORTCUTS[id]
    return {
      id,
      keys: shortcut.also
        ? `${shortcut.label} or ${shortcut.also}`
        : shortcut.label,
      description: shortcut.description,
    }
  })
</script>

<SettingsPanel data-od-id="settings-shortcuts-group">
  {#each rows as row (row.id)}
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
