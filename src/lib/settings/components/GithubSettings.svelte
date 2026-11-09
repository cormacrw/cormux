<script lang="ts">
  import SettingsPanel from '$lib/settings/components/SettingsPanel.svelte'
  import SettingsRow from '$lib/settings/components/SettingsRow.svelte'
  import { prs } from '$lib/state'
</script>

<SettingsPanel data-od-id="settings-github">
  <SettingsRow
    title="GitHub account"
    description={prs.authConfigured
      ? 'Signed in with the gh CLI. Open pull requests sync from that login.'
      : 'Run gh auth login in a terminal. Sync uses that session, not a saved token.'}
  >
    {#snippet control()}
      <span class="text-xs font-medium">
        {prs.authConfigured ? 'Connected' : 'Not connected'}
      </span>
    {/snippet}
  </SettingsRow>
</SettingsPanel>
