export type ShortcutReference = {
  keys: string
  description: string
}

export const SETTINGS_SHORTCUTS: ShortcutReference[] = [
  { keys: '⌘K', description: 'Open command palette' },
  { keys: '⌘N', description: 'New workspace' },
  { keys: '⌘S', description: 'New scratch' },
  { keys: '⌘H', description: 'Go to Homebase' },
  { keys: '⌘1–⌘9', description: 'Open a workspace, in sidebar order' },
  {
    keys: 'Ctrl+Tab',
    description:
      'Switch between workspaces and pages, most recent first (hold Ctrl to pick)',
  },
  { keys: '⌘,', description: 'Open settings' },
  { keys: '⌘L', description: 'Start a new session in the open thread' },
  { keys: '⇧⌘N', description: 'New thread (workspace)' },
  { keys: '⇧⌘] / ⇧⌘[', description: 'Next or previous tab (workspace)' },
  { keys: '⇧⌘P', description: 'Plan mode on or off (Claude thread)' },
  { keys: '⌘F', description: 'Find in the conversation' },
  {
    keys: '⌘P',
    description: 'Create or open the PR, or submit a review (workspace)',
  },
  { keys: '⌘R', description: 'Run or restart the app (workspace)' },
  { keys: '⌘.', description: 'Stop the app (workspace)' },
  { keys: '⌘B', description: 'Switch branch (workspace)' },
  { keys: '⌘I', description: 'Review the branch (workspace)' },
  { keys: '⌘T', description: 'Open the worktree in Terminal (workspace)' },
  { keys: '⌘D', description: 'Delete the workspace (workspace)' },
  { keys: '⌘J', description: 'More workspace actions (workspace)' },
  { keys: '⌘E', description: 'End scratch' },
  { keys: '⌘G', description: 'Show or hide Git (workspace)' },
  { keys: '⌘O or Ctrl+`', description: 'Show or hide Output (workspace)' },
  {
    keys: 'Esc',
    description:
      'Close popovers, changes panel, or palette; otherwise stop a working agent',
  },
  { keys: '⌘↵', description: 'Jump to the composer (thread)' },
]
