import {
  formatChangeCounts,
  type DiffLineTotals,
} from '$lib/changes/diff-totals'
import { plural } from '$lib/format/plural'

export function formatPrRouteLine(input: {
  branch: string
  base: string
  fileCount: number
  totals: DiffLineTotals
}): string {
  const { branch, base, fileCount, totals } = input
  const counts = formatChangeCounts(totals)
  const changePart =
    fileCount === 0
      ? '· No file changes yet'
      : counts
        ? `· ${plural(fileCount, 'file')} ${counts}`
        : `· ${plural(fileCount, 'file')}`
  return `${branch} → ${base} ${changePart}`
}
