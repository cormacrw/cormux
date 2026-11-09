import { describe, expect, it } from 'vitest'
import { engineStatusLine } from './engines'

describe('engineStatusLine', () => {
  it('says it is checking, not "Not installed", while detection runs', () => {
    expect(engineStatusLine({ state: 'checking' }, 'claude')).toBe('Checking…')
  })

  it("shows the core's error when detection fails", () => {
    expect(
      engineStatusLine({ state: 'failed', error: 'PATH unreadable' }, 'claude'),
    ).toBe('PATH unreadable')
  })

  it('says "Not installed" only when detection found no engine', () => {
    expect(
      engineStatusLine({ state: 'detected', statuses: [] }, 'claude'),
    ).toBe('Not installed')
    expect(
      engineStatusLine(
        {
          state: 'detected',
          statuses: [
            {
              kind: 'claude',
              installed: true,
              binary: '/usr/local/bin/claude',
              version: '2.1.0',
              signedIn: true,
            },
          ],
        },
        'claude',
      ),
    ).toBe('/usr/local/bin/claude · v2.1.0 · Signed in')
  })
})
