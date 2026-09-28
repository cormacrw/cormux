import { describe, expect, it } from 'vitest'
import { composerPauseLabel, composerShowsPauseControl } from './can-pause'

describe('composer pause controls', () => {
  it('shows pause only for running or paused threads', () => {
    expect(composerShowsPauseControl('running')).toBe(true)
    expect(composerShowsPauseControl('paused')).toBe(true)
    expect(composerShowsPauseControl('idle')).toBe(false)
    expect(composerShowsPauseControl('provisioning')).toBe(false)
  })

  it('labels resume when paused', () => {
    expect(composerPauseLabel('paused', true)).toBe('Resume')
    expect(composerPauseLabel('running', false)).toBe('Pause')
  })
})
