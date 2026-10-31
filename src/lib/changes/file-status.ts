import type { DiffFile } from '$lib/ipc/bindings'

export type FileChangeStatus = 'M' | 'A' | 'D' | 'R'

export function inferFileStatus(file: DiffFile): FileChangeStatus {
  if (file.oldPath) return 'R'
  if (file.deleted > 0 && file.added === 0) return 'D'
  if (file.added > 0 && file.deleted === 0) {
    const onlyAdds = file.hunks.every((hunk) =>
      hunk.body.split('\n').every((line) => !line.startsWith('-')),
    )
    if (onlyAdds) return 'A'
  }
  return 'M'
}

export const statusLabel: Record<FileChangeStatus, string> = {
  M: 'Modified',
  A: 'Added',
  D: 'Deleted',
  R: 'Renamed',
}

export function splitPath(path: string): { dir: string; name: string } {
  const cut = path.lastIndexOf('/')
  if (cut === -1) return { dir: '', name: path }
  return { dir: `${path.slice(0, cut + 1)}`, name: path.slice(cut + 1) }
}

const TEST_DIR = /(^|\/)(__tests__|tests?|spec|e2e)\//
const TEST_FILE =
  /[._-](test|spec)\.[^/]+$|(^|\/)test_[^/]+\.py$|_test\.(go|py|rs)$/

/** Test files start folded in Changes so the code under review comes first. */
export function isTestPath(path: string): boolean {
  return TEST_DIR.test(path) || TEST_FILE.test(path)
}

const GENERATED_FILE =
  /(^|\/)(pnpm-lock\.yaml|package-lock\.json|yarn\.lock|bun\.lockb?|Cargo\.lock|Gemfile\.lock|poetry\.lock|uv\.lock|composer\.lock|go\.sum)$|\.(min\.(js|css)|map|snap)$|\.generated\.[^/]+$|(^|\/)__generated__\//

/** Lockfiles and build output: nobody reads these line by line. */
export function isGeneratedPath(path: string): boolean {
  return GENERATED_FILE.test(path)
}

/** Past this many changed lines a diff starts folded and isn't syntax highlighted. */
export const LARGE_DIFF_LINES = 1500

export function isLargeDiff(
  file: Pick<DiffFile, 'added' | 'deleted'>,
): boolean {
  return file.added + file.deleted > LARGE_DIFF_LINES
}

/** A file is folded if the user folded it, or by default when it's a test, generated or huge. */
export function isDiffCollapsed(
  collapsed: Record<string, boolean>,
  file: Pick<DiffFile, 'path' | 'added' | 'deleted'>,
): boolean {
  return (
    collapsed[file.path] ??
    (isTestPath(file.path) || isGeneratedPath(file.path) || isLargeDiff(file))
  )
}
