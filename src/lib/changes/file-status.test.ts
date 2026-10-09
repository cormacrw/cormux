import { describe, expect, it } from 'vitest'
import { inferFileStatus, isDiffCollapsed, isTestPath } from './file-status'

describe('inferFileStatus', () => {
  it('marks deletions', () => {
    expect(
      inferFileStatus({
        path: 'gone.ts',
        oldPath: null,
        added: 0,
        deleted: 4,
        hunks: [{ header: '', body: '-x\n' }],
      }),
    ).toBe('D')
  })

  it('marks additions', () => {
    expect(
      inferFileStatus({
        path: 'new.ts',
        oldPath: null,
        added: 3,
        deleted: 0,
        hunks: [{ header: '', body: '+x\n' }],
      }),
    ).toBe('A')
  })

  it('marks renames, even with edits', () => {
    expect(
      inferFileStatus({
        path: 'lib/moved.ts',
        oldPath: 'moved.ts',
        added: 1,
        deleted: 1,
        hunks: [{ header: '', body: '-x\n+y\n' }],
      }),
    ).toBe('R')
  })
})

describe('isTestPath', () => {
  it.each([
    'src/lib/foo.test.ts',
    'src/lib/foo.spec.tsx',
    'src/__tests__/foo.ts',
    'tests/integration.rs',
    'e2e/workspace.spec.ts',
    'pkg/server_test.go',
    'app/test_models.py',
  ])('treats %s as a test', (path) => {
    expect(isTestPath(path)).toBe(true)
  })

  it.each([
    'src/lib/foo.ts',
    'src/testing-utils.ts',
    'src/contest.ts',
    'src/lib/latest.svelte',
  ])('leaves %s alone', (path) => {
    expect(isTestPath(path)).toBe(false)
  })
})

describe('isDiffCollapsed', () => {
  it('folds tests by default and respects an explicit choice', () => {
    expect(isDiffCollapsed({}, 'a.test.ts')).toBe(true)
    expect(isDiffCollapsed({}, 'a.ts')).toBe(false)
    expect(isDiffCollapsed({ 'a.test.ts': false }, 'a.test.ts')).toBe(false)
    expect(isDiffCollapsed({ 'a.ts': true }, 'a.ts')).toBe(true)
  })
})
