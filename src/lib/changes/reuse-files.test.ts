import { describe, expect, it } from 'vitest'
import type { DiffFile } from '$lib/ipc/bindings'
import { reuseUnchangedFiles } from './reuse-files'

const file = (path: string, body: string): DiffFile => ({
  path,
  added: 1,
  deleted: 0,
  hunks: [{ header: '@@ -1 +1 @@\n', body }],
})

describe('reuseUnchangedFiles', () => {
  it('keeps the old object for an unchanged file and takes the new one otherwise', () => {
    const same = file('a.ts', '+a\n')
    const changed = file('b.ts', '+b\n')
    const next = reuseUnchangedFiles(
      [same, changed],
      [file('a.ts', '+a\n'), file('b.ts', '+bb\n'), file('c.ts', '+c\n')],
    )
    expect(next[0]).toBe(same)
    expect(next[1]).not.toBe(changed)
    expect(next[1]?.hunks[0]?.body).toBe('+bb\n')
    expect(next.map((row) => row.path)).toEqual(['a.ts', 'b.ts', 'c.ts'])
  })
})
