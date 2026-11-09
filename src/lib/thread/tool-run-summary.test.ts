import { describe, expect, it } from 'vitest'
import type { ToolRole } from '$lib/ipc/bindings'
import type { ToolStep } from './timeline-types'
import { editedPaths, toolRunSummary } from './tool-run-summary'

const step = (id: string, role: ToolRole, locations: string[] = []) =>
  ({
    kind: 'tool',
    id,
    role,
    title: 'Tool',
    detail: null,
    locations,
    status: 'completed',
    seq: 1,
    atMs: 0,
  }) satisfies ToolStep

const edit = (id: string, path: string) =>
  step(id, { kind: 'edit', verb: 'Edited' }, [path])

describe('tool run summary', () => {
  const steps = [
    step('1', { kind: 'read' }),
    step('2', { kind: 'read' }),
    step('3', { kind: 'search' }),
    edit('4', 'a.ts'),
    edit('5', 'a.ts'),
    step('6', { kind: 'command', command: 'ls', description: null }),
  ]

  it('says what the run did, counting a file once', () => {
    expect(toolRunSummary(steps)).toBe(
      'Read 2 files, ran 1 search, edited 1 file, ran 1 command',
    )
    expect(editedPaths(steps)).toEqual(['a.ts'])
  })

  it('counts agents rather than calling them other steps', () => {
    const agent = step('a', { kind: 'subagent', description: null })
    expect(toolRunSummary([step('1', { kind: 'read' }), agent])).toBe(
      'Read 1 file, ran 1 agent',
    )
  })
})
