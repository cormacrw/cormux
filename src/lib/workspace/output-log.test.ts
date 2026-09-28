import { describe, expect, it } from 'vitest'
import { classifyOutputLine } from './output-log'

describe('classifyOutputLine', () => {
  it('tags commands, separators, and warnings', () => {
    expect(classifyOutputLine('> pnpm dev')).toBe('cmd')
    expect(classifyOutputLine('──── App stopped ────')).toBe('sep')
    expect(
      classifyOutputLine('Port 5173 is in use, trying another one...'),
    ).toBe('warn')
    expect(classifyOutputLine('VITE v5.4.8  ready in 612 ms')).toBe('ok')
    expect(classifyOutputLine('^C')).toBe('dim')
  })
})
