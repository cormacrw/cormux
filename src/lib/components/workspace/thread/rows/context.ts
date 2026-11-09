import type { Action } from 'svelte/action'
import type { SvelteSet } from 'svelte/reactivity'
import type { DiffFile } from '$lib/ipc/bindings'
import type { EntryScope } from '$lib/thread/entering'
import type { FindPullRequest } from '$lib/thread/enhance-markdown'

/** The workspace a conversation is in, and where its files and findings lead. */
export type ConversationWorkspace = {
  /** Whose review findings the conversation's rows count. */
  readonly id: string
  /** The thread's worktree, so media a reply references can resolve and render. */
  readonly workdir: string
  readonly diffFile: (path: string) => DiffFile | undefined
  /** Opens a file in Changes. */
  readonly revealFile: (path: string, line?: number | null) => void
  /** Opens the workspace's Findings beside this thread. */
  readonly openFindings: () => void
}

/** Where a conversation's files and links lead, which its view supplies. */
export type ConversationSource = {
  readonly findPr: FindPullRequest
  /**
   * Unset where there's no workspace (a scratch): file references stay plain text, and reads
   * (from when scratches still ran) are marked read-only.
   */
  readonly workspace: ConversationWorkspace | undefined
}

/** What every row of one conversation shares. Read through getters, so a row only tracks what it reads. */
export type RowContext = ConversationSource & {
  /** Rows new to this thread pop in and replies type out; undefined renders without motion. */
  readonly entries: EntryScope | undefined
  readonly nowMs: number
  /** What the agent is still writing: its reply keeps a cursor, its thought pulses, its run stays open. */
  readonly activeIds: {
    reply: string | null
    thought: string | null
    run: string | null
  }
  /** Folds opened in this thread, kept while rows unmount off screen. */
  readonly openFolds: SvelteSet<string>
  /** Put on each row's element with its key (`rowKey`), so the conversation can reach a drawn row. */
  readonly trackRow: Action<HTMLElement, string>
}
