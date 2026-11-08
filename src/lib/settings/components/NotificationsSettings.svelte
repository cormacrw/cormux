<script lang="ts">
  import { updateSettings } from '$lib/settings/actions'
  import { Switch } from '$lib/components/ui/switch'
  import SettingsPanel from '$lib/settings/components/SettingsPanel.svelte'
  import SettingsRow from '$lib/settings/components/SettingsRow.svelte'
  import { settings } from '$lib/settings/store.svelte'
</script>

<SettingsPanel data-od-id="settings-notifications">
  <SettingsRow
    title="Approval requests"
    description="Desktop notification when an agent needs approval while Cormux is in the background"
  >
    {#snippet control()}
      <Switch
        bind:checked={
          () => settings.values.notifyApprovals,
          (next) => void updateSettings({ notifyApprovals: next })
        }
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
        bind:checked={
          () => settings.values.notifyReviewFinished,
          (next) => void updateSettings({ notifyReviewFinished: next })
        }
        aria-label="Finished reviews"
      />
    {/snippet}
  </SettingsRow>
</SettingsPanel>
