import { describe, expect, it } from 'vitest'
import type { ToolRunStep } from './timeline-types'
import { editedPaths, toolRunSummary } from './tool-run-summary'

const tool = (
  id: string,
  icon: 'file' | 'search' | 'terminal',
  title: string,
) => ({ kind: 'tool', id, icon, title, seq: 1, atMs: 0 }) satisfies ToolRunStep
const edit = (id: string, path: string) =>
  ({
    kind: 'edit',
    id,
    path,
    verb: 'Edited',
    seq: 1,
    atMs: 0,
  }) satisfies ToolRunStep

describe('tool run summary', () => {
  const steps = [
    tool('1', 'file', 'Read'),
    tool('2', 'file', 'Read'),
    tool('3', 'search', 'Searched'),
    edit('4', 'a.ts'),
    edit('5', 'a.ts'),
    tool('6', 'terminal', 'Ran'),
  ]

  it('says what the run did, counting a file once', () => {
    expect(toolRunSummary(steps)).toBe(
      'Read 2 files, ran 1 search, edited 1 file, ran 1 command',
    )
    expect(editedPaths(steps)).toEqual(['a.ts'])
  })
})
