<script lang="ts">
  import SettingsPanel from '$lib/settings/components/SettingsPanel.svelte'
  import { Kbd, KbdGroup } from '$lib/components/ui/kbd'
  import {
    SHORTCUTS,
    shortcutLabels,
    type Shortcut,
    type ShortcutId,
  } from '$lib/keyboard/shortcuts'

  const rows = (Object.keys(SHORTCUTS) as ShortcutId[]).flatMap((id) => {
    const shortcut: Shortcut = SHORTCUTS[id]
    if (shortcut.hidden) return []
    return [
      {
        id,
        // Each way to press it, as its keys: `Ctrl+Tab` is Ctrl and Tab.
        ways: shortcutLabels(id).map((label) => label.split('+')),
        description: shortcut.description,
      },
    ]
  })
</script>

<SettingsPanel data-od-id="settings-shortcuts-group">
  {#each rows as row (row.id)}
    <div
      class="flex items-center justify-between gap-4 border-b border-border px-4 py-2.5 last:border-b-0"
    >
      <span class="text-sm text-muted-foreground">{row.description}</span>
      <KbdGroup>
        {#each row.ways as keys, way (way)}
          {#if way > 0}
            <span class="text-xs text-muted-foreground">or</span>
          {/if}
          {#each keys as part, index (part + index)}
            {#if index > 0}
              <span class="text-xs text-muted-foreground">+</span>
            {/if}
            <Kbd>{part}</Kbd>
          {/each}
        {/each}
      </KbdGroup>
    </div>
  {/each}
</SettingsPanel>
