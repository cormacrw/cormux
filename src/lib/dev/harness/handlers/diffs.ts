import type { DiffTarget, WorktreeDiff } from '$lib/ipc/bindings'
import type { HarnessRuntime } from '../runtime'
import type { HandlerSlice } from '../types'

export function diffHandlers(runtime: HarnessRuntime) {
  const { db, scenario } = runtime

  const diffFiles = () => {
    const count = scenario.bigDiff ?? 0
    if (!count) return db.diff.files
    return Array.from({ length: count }, (_, file) => {
      const lines = Array.from(
        { length: 240 },
        (_, line) =>
          `+export const value${line} = computeSomething(${line}, '${'x'.repeat(line % 40)}') // ${file}`,
      )
      return {
        path: `src/generated/module-${file}.ts`,
        oldPath: null,
        status: 'added' as const,
        added: lines.length,
        deleted: 0,
        hunks: [
          {
            header: `@@ -0,0 +1,${lines.length} @@`,
            body: `${lines.join('\n')}\n`,
          },
        ],
      }
    })
  }

  // Paths thrown away from Changes; `null` once everything has been.
  let discarded: Set<string> | null = new Set()
  const uncommittedFiles = () =>
    diffFiles().filter((file) => discarded && !discarded.has(file.path))

  const filesFor = (target: DiffTarget | null) =>
    target ? diffFiles() : uncommittedFiles()

  /** As the command returns it; the stream leaves `uncommitted` out, so the store counts the files. */
  const diffFor = (target: DiffTarget | null): WorktreeDiff => {
    const files = filesFor(target)
    return {
      ...db.diff,
      target,
      files,
      uncommitted: {
        added: files.reduce((sum, file) => sum + file.added, 0),
        deleted: files.reduce((sum, file) => sum + file.deleted, 0),
      },
    }
  }

  const sameTarget = (a: DiffTarget | null, b: DiffTarget | null) =>
    JSON.stringify(a) === JSON.stringify(b)
  // What the last diff sent showed, and what the panel last asked for.
  let sentTarget: DiffTarget | null | undefined
  let wantedTarget: DiffTarget | null = null

  const sendDiff = (target: DiffTarget | null) => {
    if (!runtime.diffChannel) return
    sentTarget = target
    runtime.send(runtime.diffChannel, {
      workspaceId: db.diff.workspaceId,
      diff: { ...db.diff, target, files: filesFor(target) },
    })
  }

  return {
    subscribe_diffs: ({
      workspaceId,
      channel,
    }: {
      workspaceId: string
      channel: { id: number }
    }) => {
      if (workspaceId === db.diff.workspaceId) {
        runtime.diffChannel = { id: channel.id, index: 0 }
        queueMicrotask(() => sendDiff(null))
      }
      return runtime.newId()
    },
    refresh_workspace_diff: () => null,
    set_workspace_diff_target: async ({
      workspaceId,
      target,
    }: {
      workspaceId: string
      target: DiffTarget | null
    }) => {
      if (workspaceId !== db.diff.workspaceId) {
        return {
          workspaceId,
          target,
          files: [],
          uncommitted: { added: 0, deleted: 0 },
        }
      }
      const next = target ?? null
      wantedTarget = next
      const ms = next ? (scenario.slowBranchDiff ?? 0) : 0
      if (ms) await new Promise((resolve) => setTimeout(resolve, ms))
      // As in the core: a compute the panel has moved on from isn't published, and an unchanged
      // diff isn't sent again. Otherwise it follows on the channel once the command has returned.
      if (
        sameTarget(wantedTarget, next) &&
        !sameTarget(sentTarget ?? null, next)
      ) {
        queueMicrotask(() => sendDiff(next))
      }
      return diffFor(next)
    },
    discard_workspace_changes: ({
      workspaceId,
      path,
    }: {
      workspaceId: string
      path: string | null
    }) => {
      if (workspaceId === db.diff.workspaceId) {
        if (path == null) discarded = null
        else discarded?.add(path)
        sendDiff(null)
      }
      return null
    },
  } satisfies HandlerSlice
}
