import type { DiffComment } from './diff-comment-format'

export type { DiffComment, DiffCommentSide } from './diff-comment-format'

/** Draft review comments on the Changes diff, per workspace, until sent to an agent. */
export class DiffCommentsStore {
  byWorkspace = $state<Record<string, DiffComment[]>>({})

  list(workspaceId: string): DiffComment[] {
    return this.byWorkspace[workspaceId] ?? []
  }

  forFile(workspaceId: string, path: string): DiffComment[] {
    return this.list(workspaceId).filter((comment) => comment.path === path)
  }

  add(workspaceId: string, comment: Omit<DiffComment, 'id'>) {
    const next = { ...comment, id: crypto.randomUUID() }
    this.byWorkspace = {
      ...this.byWorkspace,
      [workspaceId]: [...this.list(workspaceId), next],
    }
  }

  remove(workspaceId: string, id: string) {
    this.byWorkspace = {
      ...this.byWorkspace,
      [workspaceId]: this.list(workspaceId).filter((comment) => comment.id !== id),
    }
  }

  clear(workspaceId: string) {
    const next = { ...this.byWorkspace }
    delete next[workspaceId]
    this.byWorkspace = next
  }
}

export const diffComments = new DiffCommentsStore()
