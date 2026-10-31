export type DiffBlockLine = {
  kind: 'add' | 'del' | 'hunk' | 'meta' | 'context'
  text: string
}

export type DiffBlock = {
  /** The file, when the diff names one. */
  path: string | null
  added: number
  deleted: number
  lines: DiffBlockLine[]
}

function lineKind(text: string): DiffBlockLine['kind'] {
  if (/^(\+\+\+|---|diff |index )/.test(text)) return 'meta'
  if (text.startsWith('@@')) return 'hunk'
  if (text.startsWith('+')) return 'add'
  if (text.startsWith('-')) return 'del'
  return 'context'
}

export function parseDiffBlock(code: string): DiffBlock {
  const lines = code
    .replace(/\n$/, '')
    .split('\n')
    .map((text) => ({ kind: lineKind(text), text }))
  const target = /^\+\+\+ (?:b\/)?(.+)$/m.exec(code)?.[1]?.trim()
  return {
    path: target && target !== '/dev/null' ? target : null,
    added: lines.filter((line) => line.kind === 'add').length,
    deleted: lines.filter((line) => line.kind === 'del').length,
    // The header's file names are already in the caption.
    lines: lines.filter((line) => line.kind !== 'meta'),
  }
}
