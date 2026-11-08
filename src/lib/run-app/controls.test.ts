import { describe, expect, it } from 'vitest'
import { appActionAllowed, appControlsFor } from './controls'

const ready = { lifecycle: 'idle' as const, runCommand: 'pnpm dev' }

describe('appControlsFor', () => {
  it('runs a stopped app only once setup is done and there is a command', () => {
    expect(appControlsFor({ ...ready, appStatus: 'stopped' }).start).toEqual({
      action: 'run',
      enabled: true,
    })
    const noCommand = appControlsFor({
      ...ready,
      appStatus: 'stopped',
      runCommand: '',
    })
    expect(noCommand.start?.enabled).toBe(false)
    expect(appActionAllowed(noCommand, 'run')).toBe(false)
    for (const lifecycle of ['provisioning', 'provisioningFailed'] as const) {
      expect(
        appControlsFor({ ...ready, lifecycle, appStatus: 'stopped' }).start
          ?.enabled,
      ).toBe(false)
    }
  })

  it('restarts a crashed app from Play, command or not', () => {
    const crashed = appControlsFor({
      ...ready,
      appStatus: 'crashed',
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
