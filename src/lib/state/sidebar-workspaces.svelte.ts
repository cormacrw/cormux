import {
  nestThreadList,
  sortThreadList,
  splitThreadList,
  threadActivityMs,
  threadUrgency,
  type ThreadUrgency,
} from '$lib/sidebar/thread-list'
import { app } from './app.svelte'
import { repos } from './repos.svelte'
import { scratches } from './scratches.svelte'
import { threadTimeline } from './thread-timeline.svelte'
import { threads } from './threads.svelte'
import { workspaceRecords } from './workspace-records.svelte'
import { workspaces, type Workspace } from './workspaces.svelte'

export type ThreadListItem = {
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
    const rows = workspaces.liveItems.map((workspace) => {
      const thread = threads.liveFor(workspace.id)
      const events = thread
        ? (threadTimeline.eventsByThread[thread.id] ?? [])
        : []
      const repoId = workspaceRecords.getRecord(workspace.id)?.repoId
      const parentThread = thread?.parentThreadId
        ? threads.getById(thread.parentThreadId)
        : undefined
      const parent =
        parentThread && workspaces.getById(parentThread.workspaceId)
      return {
        id: workspace.id,
        title: workspace.name,
        repoName: (repoId && repos.getById(repoId)?.name) || null,
        branch: workspace.branch,
        urgency: threadUrgency({
          lifecycle: workspace.lifecycle,
          paused: workspace.paused,
          pendingApprovals: workspace.pendingApprovals,
          threadStatus: thread?.status,
        }),
        activityAtMs: threadActivityMs(events) || (workspace.createdAtMs ?? 0),
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
      activityAtMs:
        threadActivityMs(
          threadTimeline.eventsByThread[scratch.threadId] ?? [],
        ) ||
        (scratch.createdAtMs ?? 0),
      workspace: null,
      parentId: null,
      parentTitle: null,
    }))
    return sortThreadList([...rows, ...legacy])
  })

  readonly sections = $derived.by(() => {
    const { threads, working } = splitThreadList(
      this.items,
      app.view === 'workspace' ? app.workspaceId : null,
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
