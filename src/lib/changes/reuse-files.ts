import type { DiffFile } from '$lib/ipc/bindings'

function sameFile(a: DiffFile, b: DiffFile): boolean {
  return (
    a.oldPath === b.oldPath &&
    a.added === b.added &&
    a.deleted === b.deleted &&
    a.hunks.length === b.hunks.length &&
    a.hunks.every(
      (hunk, index) =>
        hunk.header === b.hunks[index]?.header &&
        hunk.body === b.hunks[index]?.body,
    )
  )
}

/** Keeps the old object for files that didn't change, so their rendered diffs aren't rebuilt. */
export function reuseUnchangedFiles(
  previous: DiffFile[] | undefined,
  next: DiffFile[],
): DiffFile[] {
  if (!previous?.length) return next
  const byPath = new Map(previous.map((file) => [file.path, file]))
  return next.map((file) => {
    const old = byPath.get(file.path)
    return old && sameFile(old, file) ? old : file
  })
}
