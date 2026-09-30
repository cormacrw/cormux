export type ShortcutReference = {
  keys: string
  description: string
}

export const SETTINGS_SHORTCUTS: ShortcutReference[] = [
  { keys: '⌘K', description: 'Open command palette' },
  { keys: '⌘N', description: 'New workspace' },
  { keys: '⌘H', description: 'Go to Homebase' },
  { keys: '⌘1–⌘9', description: 'Open a workspace, in sidebar order' },
  { keys: '⌘,', description: 'Open settings' },
  { keys: '⌘L', description: 'Start a new session in the open thread' },
  {
    keys: '⌘P',
    description: 'Create or open the PR, or submit a review (workspace)',
  },
  { keys: '⌘R', description: 'Run or restart the app (workspace)' },
  { keys: '⌘.', description: 'Stop the app (workspace)' },
  { keys: '⌘B', description: 'Switch branch (workspace)' },
  { keys: '⌘T', description: 'Open the worktree in Terminal (workspace)' },
  { keys: '⌘D', description: 'Delete the workspace (workspace)' },
  { keys: '⌘J', description: 'More workspace actions (narrow windows)' },
  { keys: '⌘E', description: 'End scratch' },
  { keys: '⌘G', description: 'Open the Git tab (workspace)' },
  { keys: 'Ctrl+`', description: 'Toggle Output tab (workspace)' },
  { keys: 'Esc', description: 'Close popovers, changes panel, or palette' },
]
