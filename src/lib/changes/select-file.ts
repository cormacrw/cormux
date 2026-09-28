import type { DiffFile } from '$lib/ipc/bindings'

export function selectedFileIndex(
  files: DiffFile[],
  selectedPath: string | null,
): number {
  if (!files.length) return 0
  if (selectedPath) {
    const idx = files.findIndex((file) => file.path === selectedPath)
    if (idx >= 0) return idx
  }
  return 0
}

export function selectedFile(
  files: DiffFile[],
  selectedPath: string | null,
): DiffFile | undefined {
  return files[selectedFileIndex(files, selectedPath)]
}
