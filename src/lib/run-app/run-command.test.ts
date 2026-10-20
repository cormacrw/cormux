import { describe, expect, it } from 'vitest'
import { overrideToSave, runCommandOverride } from './run-command'

describe('workspace run command', () => {
  it('reads the workspace’s override, trimmed', () => {
    const rows = [
      { key: 'workspace:w1/runCommand', value: ' pnpm dev -p 4000 ' },
    ]
    expect(runCommandOverride(rows, 'w1')).toBe('pnpm dev -p 4000')
    expect(runCommandOverride(rows, 'w2')).toBe('')
  })

  it('stores nothing for a blank edit or the repo’s own command', () => {
    expect(overrideToSave('pnpm dev -p 4000', 'pnpm dev')).toBe(
      'pnpm dev -p 4000',
    )
    expect(overrideToSave('  pnpm dev ', 'pnpm dev')).toBe('')
    expect(overrideToSave('   ', 'pnpm dev')).toBe('')
  })
})
