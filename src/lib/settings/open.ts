import { app } from '$lib/state/app.svelte'
import { settings } from '$lib/state/settings.svelte'

export function openSettingsSection(section: string) {
  settings.focusSection = section
  app.openSettings()
}

/** Settings › Repositories with this repo open and its run command field focused. */
export function openRepoRunCommand(repoId: string) {
  settings.expandRepoId = repoId
  settings.focusRepoRunCommand = repoId
  openSettingsSection('repos')
}
