<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import SettingsPanel from '$lib/components/settings/SettingsPanel.svelte'
  import SettingsRow from '$lib/components/settings/SettingsRow.svelte'
  import { commands } from '$lib/ipc'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { fetchSnapshot } from '$lib/ipc'
  import { hydrateFromSnapshot, prs } from '$lib/state'

  let tokenDraft = $state('')
  let busy = $state(false)

  async function saveToken() {
    const trimmed = tokenDraft.trim()
    if (!trimmed) return
    busy = true
    const result = await commands.setGithubToken(trimmed)
    busy = false
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    tokenDraft = ''
    hydrateFromSnapshot(await fetchSnapshot())
    void commands.syncPullRequests()
  }

  async function signOut() {
    busy = true
    const result = await commands.clearGithubToken()
    busy = false
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    hydrateFromSnapshot(await fetchSnapshot())
  }
</script>

<SettingsPanel data-od-id="settings-github">
  <SettingsRow
    title="GitHub account"
    description={prs.authConfigured
      ? 'Signed in with a personal access token'
      : 'Add a token to sync open pull requests'}
  >
    {#snippet control()}
      <span class="text-xs font-medium">
        {prs.authConfigured ? 'Connected' : 'Not connected'}
      </span>
    {/snippet}
  </SettingsRow>

  {#if !prs.authConfigured}
    <SettingsRow
      title="Personal access token"
      description="Needs repo scope for PR sync and review actions"
      controlId="settings-github-token"
    >
      {#snippet control()}
        <div class="flex w-full max-w-md flex-col gap-2 sm:flex-row sm:items-center">
          <Input
            id="settings-github-token"
            type="password"
            autocomplete="off"
            class="font-mono text-xs"
            placeholder="ghp_…"
            bind:value={tokenDraft}
          />
          <Button
            type="button"
            size="sm"
            disabled={busy || !tokenDraft.trim()}
            onclick={() => void saveToken()}
          >
            Save token
          </Button>
        </div>
      {/snippet}
    </SettingsRow>
  {:else}
    <SettingsRow title="Sign out" description="Clears the stored token on this machine">
      {#snippet control()}
        <Button
          type="button"
          variant="secondary"
          size="sm"
          disabled={busy}
          onclick={() => void signOut()}
        >
          Sign out
        </Button>
      {/snippet}
    </SettingsRow>
  {/if}
</SettingsPanel>

<p class="mt-2 text-xs text-muted-foreground">
  Org and PR filter preferences are not configurable yet — Homebase uses All,
  Review requested, and Yours.
</p>
