import { nav, navigate } from '$lib/core/nav.svelte'
import type { RepoField, SettingsSectionId } from '$lib/core/route'

/** Opens Settings, at a section (and a repo's field in Repositories) when given. */
export function openSettings(
  at: { section?: SettingsSectionId; repoId?: string; field?: RepoField } = {},
) {
  // Already there with nowhere in particular to go: leave focus where it is.
  if (nav.view === 'settings' && !at.section && !at.repoId) return
  navigate({ view: 'settings', ...at })
}

/**
 * Settings › Repositories with this repo open and its run command field focused. The repo is
 * where it scrolls to, so there's no section for the page to jump to as well.
 */
export function openRepoRunCommand(repoId: string) {
  openSettings({ repoId, field: 'runCommand' })
}
