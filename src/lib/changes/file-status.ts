import type { DiffFile } from '$lib/ipc/bindings'

export type FileChangeStatus = 'M' | 'A' | 'D'

export function inferFileStatus(file: DiffFile): FileChangeStatus {
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
}

export function splitPath(path: string): { dir: string; name: string } {
  const cut = path.lastIndexOf('/')
  if (cut === -1) return { dir: '', name: path }
  return { dir: `${path.slice(0, cut + 1)}`, name: path.slice(cut + 1) }
}
