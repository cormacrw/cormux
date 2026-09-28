import type { DiffFile, DiffHunk } from '$lib/ipc/bindings'
import { wordChangeRanges } from './word-diff'

export type DiffLineKind = '+' | '-' | ' '

export type PreparedDiffLine = {
  kind: DiffLineKind
  text: string
  oldLine?: number
  newLine?: number
  wordRange?: [number, number] | null
}

export type PreparedHunk = {
  header: string
  oldStart: number
  newStart: number
  ctx: string
  lines: PreparedDiffLine[]
}

const HUNK_HEADER =
  /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/

export function parseHunkHeader(header: string): {
  oldStart: number
  newStart: number
  ctx: string
} {
  const match = header.match(HUNK_HEADER)
  if (!match) {
    return { oldStart: 1, newStart: 1, ctx: '' }
  }
  return {
    oldStart: Number(match[1]),
    newStart: Number(match[2]),
    ctx: match[3]?.trim() ?? '',
  }
}

function hunkBodyLines(body: string): string[] {
  return body
    .split('\n')
    .filter((line) => line.length > 0)
    .map((line) => {
      if (line.startsWith('+') || line.startsWith('-') || line.startsWith(' ')) {
        return line
      }
      return ` ${line}`
    })
}

export function prepareHunk(hunk: DiffHunk): PreparedHunk {
  const { oldStart, newStart, ctx } = parseHunkHeader(hunk.header)
  let oldLine = oldStart
  let newLine = newStart
  const raw = hunkBodyLines(hunk.body)
  const lines: PreparedDiffLine[] = raw.map((line) => {
    const kind = line[0] as DiffLineKind
    const text = line.slice(1)
    const entry: PreparedDiffLine = { kind, text }
    if (kind === ' ') {
      entry.oldLine = oldLine++
      entry.newLine = newLine++
    } else if (kind === '-') {
      entry.oldLine = oldLine++
    } else {
      entry.newLine = newLine++
    }
    return entry
  })

  for (let i = 0; i < lines.length; ) {
    const head = lines[i]
    if (!head || head.kind !== '-') {
      i++
      continue
    }
    let j = i
    while (j < lines.length && lines[j]?.kind === '-') j++
    let k = j
    while (k < lines.length && lines[k]?.kind === '+') k++
    const delCount = j - i
    const addCount = k - j
    for (let m = 0; m < Math.min(delCount, addCount); m++) {
      const removed = lines[i + m]
      const added = lines[j + m]
      if (!removed || !added) continue
      const [ra, rb] = wordChangeRanges(removed.text, added.text)
      removed.wordRange = ra
      added.wordRange = rb
    }
    i = k
  }

  return {
    header: hunk.header,
    oldStart,
    newStart,
    ctx,
    lines,
  }
}

export function formatHunkHeader(hunk: PreparedHunk): string {
  const oldCount = hunk.lines.filter((line) => line.kind !== '+').length
  const newCount = hunk.lines.filter((line) => line.kind !== '-').length
  const ctx = hunk.ctx ? ` ${hunk.ctx}` : ''
  return `@@ -${hunk.oldStart},${oldCount} +${hunk.newStart},${newCount} @@${ctx}`
}

export function prepareFile(file: DiffFile): PreparedHunk[] {
  return file.hunks.map(prepareHunk)
}

export function splitDiffRows(
  lines: PreparedDiffLine[],
): [PreparedDiffLine | null, PreparedDiffLine | null][] {
  const out: [PreparedDiffLine | null, PreparedDiffLine | null][] = []
  let i = 0
  while (i < lines.length) {
    const row = lines[i]
    if (!row) break
    if (row.kind === ' ') {
      out.push([row, row])
      i++
      continue
    }
    const dels: PreparedDiffLine[] = []
    const adds: PreparedDiffLine[] = []
    while (i < lines.length && lines[i]?.kind === '-') {
      const del = lines[i]
      if (!del) break
      dels.push(del)
      i++
    }
    while (i < lines.length && lines[i]?.kind === '+') {
      const add = lines[i]
      if (!add) break
      adds.push(add)
      i++
    }
    const max = Math.max(dels.length, adds.length)
    for (let k = 0; k < max; k++) {
      out.push([dels[k] ?? null, adds[k] ?? null])
    }
  }
  return out
}

const preparedCache = new WeakMap<DiffFile, PreparedHunk[]>()

export function cachedPreparedFile(file: DiffFile): PreparedHunk[] {
  let cached = preparedCache.get(file)
  if (!cached) {
    cached = prepareFile(file)
    preparedCache.set(file, cached)
  }
  return cached
}
