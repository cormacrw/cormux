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

/** A file is folded if the user folded it, or by default when it's a test. */
export function isDiffCollapsed(
  collapsed: Record<string, boolean>,
  path: string,
): boolean {
  return collapsed[path] ?? isTestPath(path)
}
