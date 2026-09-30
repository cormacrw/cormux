import { openPath, revealItemInDir } from '@tauri-apps/plugin-opener'
import { toastCoreError } from '$lib/feedback/wire-feedback'

export async function openWorktreeInEditor(worktreePath: string) {
  if (!worktreePath.trim()) return
  try {
    await openPath(worktreePath)
  } catch (error) {
    toastCoreError(error)
  }
}

export async function revealWorktreeInFinder(worktreePath: string) {
  if (!worktreePath.trim()) return
  try {
    await revealItemInDir(worktreePath)
  } catch (error) {
    toastCoreError(error)
  }
}
