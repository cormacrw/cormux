import { nav } from '$lib/core/nav.svelte'
import {
  nestThreadList,
  sortThreadList,
  splitThreadList,
  threadUrgency,
  type ThreadUrgency,
} from '$lib/sidebar/thread-list'
import { repos } from '$lib/state/repos.svelte'
import { scratches } from '$lib/state/scratches.svelte'
import { threadTimeline } from '$lib/thread/timeline-store.svelte'
import { threads } from '$lib/state/threads.svelte'
import { workspaces, type Workspace } from '$lib/state/workspaces.svelte'

type ThreadListItem = {
  /** The workspace id, or the scratch id for a read-only scratch. */
  id: string
  title: string
  repoName: string | null
  branch: string | null
  urgency: ThreadUrgency
  activityAtMs: number
  /** Null for a scratch, which is kept to read. */
  workspace: Workspace | null
  /** The workspace whose thread started this one, and what it's called. */
  parentId: string | null
  parentTitle: string | null
}

/** A row in the list, which `nested` draws under its parent. */
export type ThreadListRowItem = ThreadListItem & { nested: boolean }

/**
 * The sidebar's list: every thread, what needs you first, with working threads in a section of
 * their own. ⌘1–⌘9 and the Ctrl+Tab list follow the workspaces in it, collapsed or not.
 */
class ThreadList {
  readonly items = $derived.by((): ThreadListItem[] => {
    const activity = threadTimeline.activityByThread
    const rows = workspaces.liveItems.map((workspace) => {
      const thread = threads.liveFor(workspace.id)
      const parentThread = thread?.parentThreadId
        ? threads.getById(thread.parentThreadId)
        : undefined
      const parent =
        parentThread && workspaces.getById(parentThread.workspaceId)
      return {
        id: workspace.id,
        title: workspace.name,
        repoName: repos.getById(workspace.repoId)?.name || null,
        branch: workspace.branch,
        urgency: threadUrgency({
          lifecycle: workspace.lifecycle,
          paused: workspace.paused,
          pendingApprovals: workspace.pendingApprovals,
          threadStatus: thread?.status,
        }),
        activityAtMs:
          (thread && activity[thread.id]) || (workspace.createdAtMs ?? 0),
        workspace,
        parentId: parent?.id ?? null,
        parentTitle: parent?.name ?? null,
      }
    })
    const legacy = scratches.items.map((scratch) => ({
      id: scratch.id,
      title: scratch.title,
      repoName: repos.getById(scratch.repoId)?.name ?? scratch.repoId,
      branch: null,
      urgency: 'quiet' as const,
      activityAtMs: activity[scratch.threadId] || (scratch.createdAtMs ?? 0),
      workspace: null,
      parentId: null,
      parentTitle: null,
    }))
    return sortThreadList([...rows, ...legacy])
  })

  readonly sections = $derived.by(() => {
    const { threads, working } = splitThreadList(
      this.items,
      nav.view === 'workspace' ? nav.workspaceId : null,
    )
    return {
      threads: nestThreadList(threads),
      working: nestThreadList(working),
    }
  })

  readonly ordered = $derived(
    [...this.sections.threads, ...this.sections.working].flatMap((item) =>
      item.workspace ? [item.workspace] : [],
    ),
  )
}

export const sidebarWorkspaces = new ThreadList()
