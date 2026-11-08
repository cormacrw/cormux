export type Shortcut = {
  /** What it does, for Settings › Keyboard shortcuts. */
  description: string
  /** How it's written on buttons, menus and in Settings. */
  label: string
  /** Another way to press it, listed beside `label` in Settings. */
  also?: string
  /** ⌘ (Ctrl off the Mac) with this key, matched by `isShortcut`. Unset for ones handled by hand. */
  key?: string
  shift?: boolean
}

/** Every keyboard shortcut, in the order Settings lists them. */
export const SHORTCUTS = {
  palette: { label: '⌘K', key: 'k', description: 'Open command palette' },
  newThread: { label: '⌘N', key: 'n', description: 'New thread' },
  openThread: {
    label: '⌘1–⌘9',
    description: 'Open a thread, in sidebar order',
  },
  switcher: {
    label: 'Ctrl+Tab',
    description:
      'Switch between threads and pages, most recent first (hold Ctrl to pick)',
  },
  settings: { label: '⌘,', key: ',', description: 'Open settings' },
  newSession: {
    label: '⌘L',
    key: 'l',
    description: 'Start a new session in the open thread',
  },
  find: { label: '⌘F', key: 'f', description: 'Find in the conversation' },
  pickRepo: {
    label: '⌘⇧R',
    key: 'r',
    shift: true,
    description: 'Pick the repo (new thread)',
  },
  pickModel: {
    label: '⌘⇧M',
    key: 'm',
    shift: true,
    description: 'Pick the model (thread)',
  },
  pickEffort: {
    label: '⌘⇧E',
    key: 'e',
    shift: true,
    description: 'Pick the effort (thread)',
  },
  primaryAction: {
    label: '⌘P',
    key: 'p',
    description: 'Create or open the PR, or submit a review (workspace)',
  },
  runApp: {
    label: '⌘R',
    key: 'r',
    description: 'Run or restart the app (workspace)',
  },
  stopApp: { label: '⌘.', key: '.', description: 'Stop the app (workspace)' },
  switchBranch: {
    label: '⌘B',
    key: 'b',
    description: 'Switch branch (workspace)',
  },
  reviewBranch: {
    label: '⌘I',
    key: 'i',
    description: 'Review the branch (workspace)',
  },
  openTerminal: {
    label: '⌘T',
    key: 't',
    description: 'Open the worktree in Terminal (workspace)',
  },
  deleteWorkspace: {
    label: '⌘D',
    key: 'd',
    description: 'Delete the workspace (workspace)',
  },
  moreActions: {
    label: '⌘J',
    key: 'j',
    description: 'More workspace actions (workspace)',
  },
  deleteScratch: {
    label: '⌘E',
    key: 'e',
    description: 'Delete a read-only scratch',
  },
  toggleGit: {
    label: '⌘G',
    key: 'g',
    description: 'Show or hide Git (workspace)',
  },
  toggleOutput: {
    label: '⌘O',
    also: 'Ctrl+`',
    key: 'o',
    description: 'Show or hide Output (workspace)',
  },
  escape: {
    label: 'Esc',
    description:
      'Close popovers, changes panel, or palette; otherwise stop a working agent',
  },
  composer: { label: '⌘↵', description: 'Jump to the composer (thread)' },
} satisfies Record<string, Shortcut>

export type ShortcutId = keyof typeof SHORTCUTS

/** The shortcuts `isShortcut` can match: the ones with a key. */
export type MatchedShortcutId = {
  [K in ShortcutId]: (typeof SHORTCUTS)[K] extends { key: string } ? K : never
}[ShortcutId]

export function shortcutLabel(id: ShortcutId): string {
  return SHORTCUTS[id].label
}

/** For `aria-keyshortcuts`: `Meta+Shift+R`. */
export function shortcutAria(id: MatchedShortcutId): string {
  const shortcut: Shortcut = SHORTCUTS[id]
  const key = shortcut.key!.toUpperCase()
  return shortcut.shift ? `Meta+Shift+${key}` : `Meta+${key}`
}

/** ⌘ plus the shortcut's key, Shift exactly as it says, and no Option. */
export function isShortcut(
  event: Pick<
    KeyboardEvent,
    'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'
  >,
  id: MatchedShortcutId,
): boolean {
  const shortcut: Shortcut = SHORTCUTS[id]
  return (
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    event.shiftKey === (shortcut.shift ?? false) &&
    event.key.toLowerCase() === shortcut.key
  )
}
