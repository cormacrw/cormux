export type ShortcutReference = {
  keys: string
  description: string
}

export const SETTINGS_SHORTCUTS: ShortcutReference[] = [
  { keys: '⌘K', description: 'Open command palette' },
  { keys: '⌘N', description: 'New workspace' },
  { keys: '⌘,', description: 'Open settings' },
  { keys: '⌘L', description: 'Start a new session in the open thread' },
  { keys: 'Ctrl+`', description: 'Toggle Output tab (workspace)' },
  { keys: 'Esc', description: 'Close popovers, changes panel, or palette' },
]
