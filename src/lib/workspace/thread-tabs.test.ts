import { describe, expect, it } from 'vitest'
import { findingsTabAriaLabel, outputTabAriaLabel } from './thread-tabs'
import { appStatusView } from '$lib/run-app/status'

describe('thread tab labels', () => {
  it('formats findings and output labels', () => {
    expect(findingsTabAriaLabel(3)).toBe('Review findings, 3 open findings')
    const app = { appStatus: 'running' as const, port: 5173, exitCode: null }
    expect(outputTabAriaLabel(appStatusView(app, false))).toBe(
      'App output, running on localhost:5173',
    )
    expect(
      outputTabAriaLabel(
        appStatusView({ ...app, appStatus: 'crashed', exitCode: 1 }, false),
      ),
    ).toBe('App output, crashed')
    expect(
      outputTabAriaLabel(
        appStatusView({ ...app, appStatus: 'stopped', port: null }, true),
      ),
    ).toBe('App output, setting up')
  })
})
