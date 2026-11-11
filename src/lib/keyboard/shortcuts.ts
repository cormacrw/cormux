export type Shortcut = {
  /** What it does, for Settings › Keyboard shortcuts. */
  description: string
  /** How it's written on buttons, menus and in Settings. */
  label: string
  /** ⌘ (Ctrl off the Mac) with this key, or any of these keys, matched by `isShortcut`. */
  key?: string | readonly string[]
  /** Shift exactly as given; `'either'` takes both (Ctrl+Tab goes back with ⇧). */
  shift?: boolean | 'either'
  /** Ctrl (never ⌘) with this key: a second way to press it, or the only one without `key`. */
  ctrlKey?: string
  /** Matched, but not listed in Settings. */
  hidden?: boolean
}

/** Every keyboard shortcut, in the order Settings lists them. */
export const SHORTCUTS = {
  palette: { label: '⌘K', key: 'k', description: 'Open command palette' },
  newThread: { label: '⌘N', key: 'n', description: 'New thread' },
  openThread: {
    label: '⌘1–⌘9',
    key: ['1', '2', '3', '4', '5', '6', '7', '8', '9'],
    description: 'Open a thread, in sidebar order',
  },
  switcher: {
    label: 'Ctrl+Tab',
    ctrlKey: 'Tab',
    shift: 'either',
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
  openEditor: {
    label: '⌘⇧O',
    key: 'o',
    shift: true,
    description: 'Open the worktree in your editor (workspace)',
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
    key: 'o',
    ctrlKey: '`',
    description: 'Show or hide Output (workspace)',
  },
  escape: {
    label: 'Esc',
    description:
      'Close popovers, changes panel, or palette; otherwise stop a working agent',
  },
  composer: {
    label: '⌘↵',
    key: 'Enter',
    description: 'Jump to the composer (thread)',
  },
  // ⌘S would open the webview's save dialog.
  save: { label: '⌘S', key: 's', description: 'Nothing', hidden: true },
} satisfies Record<string, Shortcut>

export type ShortcutId = keyof typeof SHORTCUTS

export type MatchedShortcutId = {
  [K in ShortcutId]: (typeof SHORTCUTS)[K] extends
    { key: unknown } | { ctrlKey: string }
    ? K
    : never
}[ShortcutId]

/** Buttons, tooltips and aria labels only ever show these: one key each. */
type SingleKeyShortcutId = {
  [K in ShortcutId]: (typeof SHORTCUTS)[K] extends { key: string } ? K : never
}[ShortcutId]

export function shortcutLabel(id: ShortcutId): string {
  return SHORTCUTS[id].label
}

/** Every way to press it, as Settings lists them: `['⌘O', 'Ctrl+`']`. */
export function shortcutLabels(id: ShortcutId): string[] {
  const shortcut: Shortcut = SHORTCUTS[id]
  return shortcut.key && shortcut.ctrlKey
    ? [shortcut.label, `Ctrl+${shortcut.ctrlKey}`]
    : [shortcut.label]
}

/** For `aria-keyshortcuts`: `Meta+Shift+R`. */
export function shortcutAria(id: SingleKeyShortcutId): string {
  const shortcut: Shortcut = SHORTCUTS[id]
  const key = (shortcut.key as string).toUpperCase()
  return shortcut.shift ? `Meta+Shift+${key}` : `Meta+${key}`
}

type KeyEvent = Pick<
  KeyboardEvent,
  'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'
>

/** ⌘ plus the shortcut's key (or Ctrl plus its `ctrlKey`), Shift as it says, and no Option. */
export function isShortcut(event: KeyEvent, id: MatchedShortcutId): boolean {
  const shortcut: Shortcut = SHORTCUTS[id]
  if (event.altKey) return false
  const shift = shortcut.shift ?? false
  if (shift !== 'either' && event.shiftKey !== shift) return false
  const key = event.key.toLowerCase()
  const keys = shortcut.key == null ? [] : [shortcut.key].flat()
  if (
    (event.metaKey || event.ctrlKey) &&
    keys.some((k) => k.toLowerCase() === key)
  )
    return true
  return (
    event.ctrlKey &&
    !event.metaKey &&
    shortcut.ctrlKey != null &&
    shortcut.ctrlKey.toLowerCase() === key
  )
}
