import {
  formatChangeCounts,
  type DiffLineTotals,
} from '$lib/workspace/diff-totals'

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
        ? `· ${fileCount} ${fileCount === 1 ? 'file' : 'files'} ${counts}`
        : `· ${fileCount} ${fileCount === 1 ? 'file' : 'files'}`
  return `${branch} → ${base} ${changePart}`
}
