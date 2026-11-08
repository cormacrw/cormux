import { describe, expect, it } from 'vitest'
import {
  LARGE_DIFF_LINES,
  isDiffCollapsed,
  isGeneratedPath,
  isTestPath,
} from './file-status'

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

const small = (path: string) => ({ path, added: 1, deleted: 1 })

describe('isGeneratedPath', () => {
  it('spots lockfiles and build output, not lookalikes', () => {
    expect(isGeneratedPath('src-tauri/Cargo.lock')).toBe(true)
    expect(isGeneratedPath('vendor/app.min.js')).toBe(true)
    expect(isGeneratedPath('src/api.generated.ts')).toBe(true)
    expect(isGeneratedPath('src/lockfile.ts')).toBe(false)
  })
})

describe('isDiffCollapsed', () => {
  it('folds tests by default and respects an explicit choice', () => {
    expect(isDiffCollapsed({}, small('a.test.ts'))).toBe(true)
    expect(isDiffCollapsed({}, small('a.ts'))).toBe(false)
    expect(isDiffCollapsed({ 'a.test.ts': false }, small('a.test.ts'))).toBe(
      false,
    )
    expect(isDiffCollapsed({ 'a.ts': true }, small('a.ts'))).toBe(true)
  })

  it('folds generated and huge files until asked', () => {
    expect(isDiffCollapsed({}, small('pnpm-lock.yaml'))).toBe(true)
    const huge = { path: 'a.ts', added: LARGE_DIFF_LINES, deleted: 1 }
    expect(isDiffCollapsed({}, huge)).toBe(true)
    expect(isDiffCollapsed({ 'a.ts': false }, huge)).toBe(false)
  })
})
