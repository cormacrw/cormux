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

  it('names dev builds in the title', () => {
    expect(resolveWindowTitle('homebase', null, undefined, 'Cormux Dev')).toBe(
      'Cormux Dev · Homebase',
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

  it('titles an open scratch by its title', () => {
    expect(
      resolveWindowTitle(
        'scratch',
        'scratch-1',
        'Why the webhook signature fails',
      ),
    ).toBe('Cormux · Why the webhook signature fails')
  })
})
