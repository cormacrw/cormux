import { describe, expect, it } from 'vitest'
import { appActionAllowed, appControlsFor, outputBody } from './controls'

const ready = { runBlock: null, runCommand: 'pnpm dev' }

describe('appControlsFor', () => {
  it("runs a stopped app unless the core says why it can't", () => {
    expect(appControlsFor({ ...ready, appStatus: 'stopped' }).start).toEqual({
      action: 'run',
      enabled: true,
    })
    const blocked = appControlsFor({
      ...ready,
      appStatus: 'stopped',
      runBlock: 'Wait for setup to finish to run the app.',
    })
    expect(blocked.start?.enabled).toBe(false)
    expect(appActionAllowed(blocked, 'run')).toBe(false)
  })

  it('restarts a crashed app from Play, command or not', () => {
    const crashed = appControlsFor({
      ...ready,
      appStatus: 'crashed',
      runBlock: 'No run command for this workspace.',
      runCommand: '',
    })
    expect(crashed.start).toEqual({ action: 'restart', enabled: true })
    expect(appActionAllowed(crashed, 'restart')).toBe(true)
    expect(appActionAllowed(crashed, 'stop')).toBe(false)
  })

  it('stops a starting app but only restarts a running one', () => {
    const starting = appControlsFor({ ...ready, appStatus: 'starting' })
    expect(starting.start).toBeNull()
    expect(appActionAllowed(starting, 'stop')).toBe(true)
    expect(appActionAllowed(starting, 'restart')).toBe(false)
    const running = appControlsFor({ ...ready, appStatus: 'running' })
    expect(appActionAllowed(running, 'restart')).toBe(true)
    expect(appActionAllowed(running, 'run')).toBe(false)
  })
})

describe('outputBody', () => {
  const stopped = {
    provisioning: false,
    appStatus: 'stopped' as const,
    runCommand: 'pnpm dev',
    hasLines: false,
  }

  it('explains an empty log only while nothing is running', () => {
    expect(outputBody(stopped)).toBe('idle')
    expect(outputBody({ ...stopped, runCommand: '' })).toBe('noCommand')
    expect(outputBody({ ...stopped, hasLines: true })).toBe('log')
    expect(outputBody({ ...stopped, provisioning: true })).toBe('log')
  })

  it('keeps the log for a running app whose output was just cleared', () => {
    expect(outputBody({ ...stopped, appStatus: 'running' })).toBe('log')
    expect(outputBody({ ...stopped, appStatus: 'starting' })).toBe('log')
    expect(outputBody({ ...stopped, appStatus: 'crashed' })).toBe('idle')
  })
})
