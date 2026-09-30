<script lang="ts">
  import { Switch } from '$lib/components/ui/switch'
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import { settings } from '$lib/state'
</script>

<SettingsPanel data-od-id="settings-notifications">
  <SettingsRow
    title="Approval requests"
    description="Desktop notification when an agent needs approval while Cormux is in the background"
  >
    {#snippet control()}
      <Switch
        checked={settings.notifyApprovals}
        onCheckedChange={(next) => void settings.setNotifyApprovals(next)}
        aria-label="Approval requests"
      />
    {/snippet}
  </SettingsRow>

  <SettingsRow
    title="Finished reviews"
    description="Desktop notification when a review workspace finishes with findings"
  >
    {#snippet control()}
      <Switch
        checked={settings.notifyReviewFinished}
        onCheckedChange={(next) => void settings.setNotifyReviewFinished(next)}
        aria-label="Finished reviews"
      />
    {/snippet}
  </SettingsRow>
</SettingsPanel>
