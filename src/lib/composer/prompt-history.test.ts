import { describe, expect, it } from 'vitest'
import type { AgentEvent } from '$lib/ipc/bindings'
import { promptHistory, stepPromptHistory } from './prompt-history'

const user = (text: string): { event: AgentEvent } => ({
  event: { type: 'messageChunk', role: 'user', text },
})
const agent = (text: string): { event: AgentEvent } => ({
  event: { type: 'messageChunk', role: 'agent', text },
})

describe('promptHistory', () => {
  it('lists user prompts oldest first, joining back-to-back chunks', () => {
    expect(
      promptHistory([
        user('Hello '),
        user('world'),
        agent('Hi'),
        user('  '),
        agent('?'),
        user('Fix it'),
      ]),
    ).toEqual(['Hello world', 'Fix it'])
  })
})

describe('stepPromptHistory', () => {
  it('walks back to the oldest prompt and forward to an empty composer', () => {
    expect(stepPromptHistory(null, 2, 'older')).toBe(1)
    expect(stepPromptHistory(1, 2, 'older')).toBe(0)
    expect(stepPromptHistory(0, 2, 'older')).toBeUndefined()
    expect(stepPromptHistory(0, 2, 'newer')).toBe(1)
    expect(stepPromptHistory(1, 2, 'newer')).toBeNull()
    expect(stepPromptHistory(null, 2, 'newer')).toBeUndefined()
  })

  it('does nothing without history', () => {
    expect(stepPromptHistory(null, 0, 'older')).toBeUndefined()
  })
})
