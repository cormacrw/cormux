import type { SvelteSet } from 'svelte/reactivity'
import type { DiffFile } from '$lib/ipc/bindings'
import type { EntryScope } from '$lib/thread/entering'
import type { FindPullRequest } from '$lib/thread/enhance-markdown'

/** What every row of one conversation shares. Read through getters, so a row only tracks what it reads. */
export type RowContext = {
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
  /** The thread's worktree, so media a reply references can resolve and render. */
  readonly workdir: string | undefined
  readonly diffFile: (path: string) => DiffFile | undefined
  readonly findPr: FindPullRequest
  /** A scratch's reads are marked read-only, with no worktree. */
  readonly scratch: boolean
}
