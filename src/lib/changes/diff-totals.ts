import type { DiffFile } from '$lib/ipc/bindings'

export type DiffLineTotals = {
  added: number
  deleted: number
}

export function totalsFromDiffFiles(files: DiffFile[]): DiffLineTotals {
  let added = 0
  let deleted = 0
  for (const file of files) {
    added += file.added
    deleted += file.deleted
  }
  return { added, deleted }
}

export function formatChangeCounts(totals: DiffLineTotals): string | null {
  if (totals.added <= 0 && totals.deleted <= 0) return null
  const parts: string[] = []
  if (totals.added > 0) parts.push(`+${totals.added}`)
  if (totals.deleted > 0) parts.push(`−${totals.deleted}`)
  return parts.join(' ')
}
