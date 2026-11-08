import { describe, expect, it } from 'vitest'
import {
  OUTPUT_LINE_CAP,
  appendOutputLines,
  classifyOutputLine,
} from './output-log'

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

describe('appendOutputLines', () => {
  it('keeps the newest lines up to the cap', () => {
    const line = (id: number) => ({ id, text: `${id}` })
    const old = Array.from({ length: OUTPUT_LINE_CAP }, (_, id) => line(id))
    const next = appendOutputLines(old, [line(OUTPUT_LINE_CAP)])
    expect(next).toHaveLength(OUTPUT_LINE_CAP)
    expect(next[0]?.id).toBe(1)
    expect(next.at(-1)?.id).toBe(OUTPUT_LINE_CAP)
  })
})
