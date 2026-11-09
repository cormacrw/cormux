import { describe, expect, it } from 'vitest'
import type { LogEntry } from '$lib/ipc/bindings'
import {
  OUTPUT_LINE_CAP,
  appendOutputLines,
  classifyOutputLine,
  outputLineText,
} from './output-log'

const output = (text: string): LogEntry => ({ kind: 'output', text })

describe('classifyOutputLine', () => {
  it("styles Cormux's own lines by kind", () => {
    expect(classifyOutputLine({ kind: 'command', text: 'pnpm dev' })).toBe(
      'cmd',
    )
    expect(classifyOutputLine({ kind: 'marker', text: 'App stopped' })).toBe(
      'sep',
    )
    expect(classifyOutputLine({ kind: 'interrupt', text: '^C' })).toBe('dim')
    expect(
      classifyOutputLine({ kind: 'warning', text: 'Port 8000 is taken' }),
    ).toBe('warn')
  })

  it('reads what dev tools print', () => {
    expect(
      classifyOutputLine(output('Port 5173 is in use, trying another one...')),
    ).toBe('warn')
    expect(classifyOutputLine(output('VITE v5.4.8  ready in 612 ms'))).toBe(
      'ok',
    )
    expect(classifyOutputLine(output('> not a command we ran'))).toBe('out')
    expect(classifyOutputLine(output('  '))).toBe('blank')
  })
})

describe('outputLineText', () => {
  it('draws a marker as a rule', () => {
    expect(outputLineText({ kind: 'marker', text: 'Worktree ready' })).toBe(
      '──── Worktree ready ────',
    )
  })
})

describe('appendOutputLines', () => {
  it('keeps the newest lines up to the cap', () => {
    const line = (id: number) => ({ id, ...output(`${id}`) })
    const old = Array.from({ length: OUTPUT_LINE_CAP }, (_, id) => line(id))
    const next = appendOutputLines(old, [line(OUTPUT_LINE_CAP)])
    expect(next).toHaveLength(OUTPUT_LINE_CAP)
    expect(next[0]?.id).toBe(1)
    expect(next.at(-1)?.id).toBe(OUTPUT_LINE_CAP)
  })
})
