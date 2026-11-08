import { describe, expect, it } from 'vitest'
import { onEscape, runEscape } from './escape'

describe('runEscape', () => {
  it('closes a slide-out before stopping the agent, whatever registered first', () => {
    const calls: string[] = []
    const offAgent = onEscape('agent', () => {
      calls.push('agent')
      return true
    })
    const offSlideOut = onEscape('slideOut', () => {
      calls.push('slideOut')
      return true
    })
    expect(runEscape()).toBe(true)
    expect(calls).toEqual(['slideOut'])

    offSlideOut()
    expect(runEscape()).toBe(true)
    expect(calls).toEqual(['slideOut', 'agent'])
    offAgent()
  })

  it('falls through a layer with nothing to do', () => {
    const off = onEscape('slideOut', () => false)
    expect(runEscape()).toBe(false)
    off()
  })
})
