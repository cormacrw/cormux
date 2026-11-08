<script lang="ts">
  import { Switch } from '$lib/components/ui/switch'
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import { settings } from '$lib/state'
</script>

<SettingsPanel data-od-id="settings-experimental-group">
  <SettingsRow
    title="References"
    description="Agents pin the PRs, tickets and docs they work on to a panel beside the thread. The panel switches at once; an agent that's already running keeps or lacks the tool until its next session."
  >
    {#snippet control()}
      <Switch
        checked={settings.experimentalReferences}
        onCheckedChange={(next) =>
          void settings.setExperimentalReferences(next)}
        aria-label="References"
        data-od-id="settings-experimental-references"
      />
    {/snippet}
  </SettingsRow>
</SettingsPanel>
