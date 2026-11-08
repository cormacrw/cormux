/** The command Run starts in a workspace: its own when set, else the repo's ('' for none). */
export function workspaceRunCommand(
  own: string | null | undefined,
  repoCommand: string | null | undefined,
): string {
  return own?.trim() || repoCommand?.trim() || ''
}

/** What to store for an edit: nothing when it's blank or just the repo's own command again. */
export function overrideToSave(input: string, repoCommand: string): string {
  const value = input.trim()
  return value === repoCommand.trim() ? '' : value
}
