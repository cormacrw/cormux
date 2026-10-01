import type { DiffTarget } from '$lib/ipc/bindings'
import type { DiffComment } from './diff-comment-format'

export type { DiffComment, DiffCommentSide } from './diff-comment-format'

/** Stored comments carry the level they were written on, since line numbers only hold within that diff. */
export type ScopedDiffComment = DiffComment & { branch: string | null }

/** The branch a diff target shows, or null for uncommitted changes. */
export function commentBranch(
  target: DiffTarget | null,
  currentBranch: string,
): string | null {
  if (!target) return null
  return target.head === 'HEAD' ? currentBranch : target.head
}

/** Draft review comments on the Changes diff, per workspace and branch, until sent to an agent. */
export class DiffCommentsStore {
  byWorkspace = $state<Record<string, ScopedDiffComment[]>>({})

  list(workspaceId: string): ScopedDiffComment[] {
    return this.byWorkspace[workspaceId] ?? []
  }

  forBranch(workspaceId: string, branch: string | null): ScopedDiffComment[] {
    return this.list(workspaceId).filter((comment) => comment.branch === branch)
  }

  forFile(
    workspaceId: string,
    branch: string | null,
    path: string,
  ): ScopedDiffComment[] {
    return this.forBranch(workspaceId, branch).filter(
      (comment) => comment.path === path,
    )
  }

  add(workspaceId: string, comment: Omit<ScopedDiffComment, 'id'>) {
    const next = { ...comment, id: crypto.randomUUID() }
    this.byWorkspace = {
      ...this.byWorkspace,
      [workspaceId]: [...this.list(workspaceId), next],
    }
  }

  remove(workspaceId: string, id: string) {
    this.byWorkspace = {
      ...this.byWorkspace,
      [workspaceId]: this.list(workspaceId).filter(
        (comment) => comment.id !== id,
      ),
    }
  }

  clear(workspaceId: string, branch: string | null) {
    this.byWorkspace = {
      ...this.byWorkspace,
      [workspaceId]: this.list(workspaceId).filter(
        (comment) => comment.branch !== branch,
      ),
    }
  }
}

export const diffComments = new DiffCommentsStore()
