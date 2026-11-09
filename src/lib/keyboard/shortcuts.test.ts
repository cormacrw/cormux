import { describe, expect, it } from 'vitest'
import { SHORTCUTS, isShortcut, shortcutAria, type Shortcut } from './shortcuts'

function press(
  key: string,
  init: {
    shiftKey?: boolean
    altKey?: boolean
    metaKey?: boolean
    ctrlKey?: boolean
  } = {},
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
      [shortcut.key ?? []]
        .flat()
        .map((key) => `${shortcut.shift ? '⇧' : ''}${key}`),
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

  it('matches the second way to press it, and every key of a range', () => {
    const ctrl = { metaKey: false, ctrlKey: true }
    expect(isShortcut(press('o'), 'toggleOutput')).toBe(true)
    expect(isShortcut(press('`', ctrl), 'toggleOutput')).toBe(true)
    expect(isShortcut(press('`'), 'toggleOutput')).toBe(false)
    expect(isShortcut(press('Tab', ctrl), 'switcher')).toBe(true)
    expect(
      isShortcut(press('Tab', { ...ctrl, shiftKey: true }), 'switcher'),
    ).toBe(true)
    expect(isShortcut(press('Tab'), 'switcher')).toBe(false)
    expect(isShortcut(press('3'), 'openThread')).toBe(true)
    expect(isShortcut(press('0'), 'openThread')).toBe(false)
    expect(isShortcut(press('Enter'), 'composer')).toBe(true)
  })
})
