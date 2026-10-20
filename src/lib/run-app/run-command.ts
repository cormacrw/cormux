import type { SettingRow } from '$lib/ipc/bindings'

/** A workspace's own run command, set in its Output tab. The core reads the same setting. */
export function runCommandOverrideKey(workspaceId: string): string {
  return `workspace:${workspaceId}/runCommand`
}

export function runCommandOverride(
  settings: SettingRow[],
  workspaceId: string,
): string {
  const key = runCommandOverrideKey(workspaceId)
  return settings.find((row) => row.key === key)?.value.trim() ?? ''
}

/** The command Run starts in a workspace: its own when set, else the repo's ('' for none). */
export function workspaceRunCommand(
  settings: SettingRow[],
  workspaceId: string,
  repoCommand: string | null | undefined,
): string {
  return runCommandOverride(settings, workspaceId) || repoCommand?.trim() || ''
}

/** What to store for an edit: nothing when it's blank or just the repo's own command again. */
export function overrideToSave(input: string, repoCommand: string): string {
  const value = input.trim()
  return value === repoCommand.trim() ? '' : value
}
