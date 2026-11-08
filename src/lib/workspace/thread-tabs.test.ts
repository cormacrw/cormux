import { describe, expect, it } from 'vitest'
import { findingsTabAriaLabel, outputTabAriaLabel } from './thread-tabs'
import { appStatusView } from '$lib/run-app/status'
import { IDLE_RUNTIME } from '$lib/state/workspace-model'

describe('thread tab labels', () => {
  it('formats findings and output labels', () => {
    expect(findingsTabAriaLabel(3)).toBe('Review findings, 3 open findings')
    const runtime = {
      ...IDLE_RUNTIME,
      appStatus: 'running' as const,
      port: 5173,
    }
    const ready = { lifecycle: 'ready' as const, runtime }
    expect(outputTabAriaLabel(appStatusView(ready))).toBe(
      'App output, running on localhost:5173',
    )
    expect(
      outputTabAriaLabel(
        appStatusView({
          ...ready,
          runtime: { ...runtime, appStatus: 'crashed', exitCode: 1 },
        }),
      ),
    ).toBe('App output, crashed')
    expect(
      outputTabAriaLabel(
        appStatusView({
          lifecycle: 'provisioning',
          runtime: { ...runtime, appStatus: 'stopped', port: null },
        }),
      ),
    ).toBe('App output, setting up')
  })
})
