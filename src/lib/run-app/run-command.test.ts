import { describe, expect, it } from 'vitest'
import { overrideToSave, workspaceRunCommand } from './run-command'

describe('workspace run command', () => {
  it('runs the workspace’s own command, else the repo’s', () => {
    expect(workspaceRunCommand(' pnpm dev -p 4000 ', 'pnpm dev')).toBe(
      'pnpm dev -p 4000',
    )
    expect(workspaceRunCommand(null, ' pnpm dev ')).toBe('pnpm dev')
    expect(workspaceRunCommand('  ', null)).toBe('')
  })

  it('stores nothing for a blank edit or the repo’s own command', () => {
    expect(overrideToSave('pnpm dev -p 4000', 'pnpm dev')).toBe(
      'pnpm dev -p 4000',
    )
    expect(overrideToSave('  pnpm dev ', 'pnpm dev')).toBe('')
    expect(overrideToSave('   ', 'pnpm dev')).toBe('')
  })
})
