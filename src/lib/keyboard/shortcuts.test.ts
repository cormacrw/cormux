import { describe, expect, it } from 'vitest'
import { SHORTCUTS, isShortcut, shortcutAria, type Shortcut } from './shortcuts'

function press(
  key: string,
  init: { shiftKey?: boolean; altKey?: boolean } = {},
) {
  return {
    key,
    metaKey: true,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    ...init,
  }
}

describe('SHORTCUTS', () => {
  it('gives each handled shortcut its own key and Shift', () => {
    const shortcuts: Shortcut[] = Object.values(SHORTCUTS)
    const combos = shortcuts.flatMap((shortcut) =>
      shortcut.key ? [`${shortcut.shift ? '⇧' : ''}${shortcut.key}`] : [],
    )
    expect(new Set(combos).size).toBe(combos.length)
  })

  it('tells ⌘R from ⌘⇧R', () => {
    expect(isShortcut(press('r'), 'runApp')).toBe(true)
    expect(isShortcut(press('R', { shiftKey: true }), 'runApp')).toBe(false)
    expect(isShortcut(press('R', { shiftKey: true }), 'pickRepo')).toBe(true)
    expect(isShortcut(press('r', { altKey: true }), 'runApp')).toBe(false)
    expect(shortcutAria('pickRepo')).toBe('Meta+Shift+R')
  })
})
