import type { Action } from 'svelte/action'
import type { SvelteSet } from 'svelte/reactivity'
import type { DiffFile } from '$lib/ipc/bindings'
import type { EntryScope } from '$lib/thread/entering'
import type { FindPullRequest } from '$lib/thread/enhance-markdown'

/** Where a conversation's files and links lead, which its view supplies. */
export type ConversationSource = {
  /** The thread's worktree, so media a reply references can resolve and render. */
  readonly workdir: string | undefined
  readonly diffFile: (path: string) => DiffFile | undefined
  readonly findPr: FindPullRequest
  /** Opens a file in Changes; unset where there's no workspace (a scratch), so none are links. */
  readonly revealFile:
    ((path: string, line?: number | null) => void) | undefined
  /** The workspace the conversation is in, whose review findings its rows count. */
  readonly workspaceId: string | undefined
  /** Opens that workspace's Findings beside this thread; unset where there's no workspace. */
  readonly openFindings: (() => void) | undefined
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
  /**
   * No workspace behind it: a scratch, whose reads (from when scratches still ran) are marked
   * read-only.
   */
  readonly scratch: boolean
}
