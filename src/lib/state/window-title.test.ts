import { describe, expect, it } from 'vitest'
import { resolveWindowTitle } from './window-title'

describe('resolveWindowTitle', () => {
  it('titles Homebase and Settings', () => {
    expect(resolveWindowTitle('homebase', null, undefined)).toBe(
      'Cormux · Homebase',
    )
    expect(resolveWindowTitle('settings', null, undefined)).toBe(
      'Cormux · Settings',
    )
  })

  it('titles an open workspace by name', () => {
    expect(
      resolveWindowTitle('workspace', 'ws-1', 'Auth session timeout'),
    ).toBe('Cormux · Auth session timeout')
  })

  it('falls back to the workspace id when name is missing', () => {
    expect(resolveWindowTitle('workspace', 'ws-1', undefined)).toBe(
      'Cormux · ws-1',
    )
  })
})
