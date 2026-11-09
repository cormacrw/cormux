import { describe, expect, it } from 'vitest'
import { isDiffCollapsed } from './file-status'

const small = (path: string) => ({ path, added: 1, deleted: 1 })
const foldedByDefault = (path: string) => isDiffCollapsed({}, small(path))

describe('isDiffCollapsed', () => {
  it.each([
    'src/lib/foo.test.ts',
    'src/lib/foo.spec.tsx',
    'src/__tests__/foo.ts',
    'tests/integration.rs',
    'e2e/workspace.spec.ts',
    'pkg/server_test.go',
    'app/test_models.py',
  ])('folds the test file %s', (path) => {
    expect(foldedByDefault(path)).toBe(true)
  })

  it.each([
    'src-tauri/Cargo.lock',
    'pnpm-lock.yaml',
    'vendor/app.min.js',
    'src/api.generated.ts',
  ])('folds the generated file %s', (path) => {
    expect(foldedByDefault(path)).toBe(true)
  })

  it.each([
    'src/lib/foo.ts',
    'src/testing-utils.ts',
    'src/contest.ts',
    'src/lib/latest.svelte',
    'src/lockfile.ts',
  ])('leaves the source file %s open', (path) => {
    expect(foldedByDefault(path)).toBe(false)
  })

  it('folds a huge diff', () => {
    expect(
      isDiffCollapsed({}, { path: 'a.ts', added: 1400, deleted: 200 }),
    ).toBe(true)
  })

  it('respects an explicit choice either way', () => {
    expect(isDiffCollapsed({ 'a.test.ts': false }, small('a.test.ts'))).toBe(
      false,
    )
    expect(isDiffCollapsed({ 'a.ts': true }, small('a.ts'))).toBe(true)
  })
})
