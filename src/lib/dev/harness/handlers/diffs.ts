import type { DiffTarget } from '$lib/ipc/bindings'
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

  const sendDiff = (target: DiffTarget | null) => {
    if (!runtime.diffChannel) return
    runtime.send(runtime.diffChannel, {
      workspaceId: db.diff.workspaceId,
      diff: {
        ...db.diff,
        target,
        files: target ? diffFiles() : uncommittedFiles(),
      },
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
    set_workspace_diff_target: ({
      workspaceId,
      target,
    }: {
      workspaceId: string
      target: DiffTarget | null
    }) => {
      // The new diff follows on the channel once the command has returned.
      if (workspaceId === db.diff.workspaceId) {
        queueMicrotask(() => sendDiff(target ?? null))
      }
      return null
    },
    discard_workspace_changes: ({
      workspaceId,
      paths,
    }: {
      workspaceId: string
      paths: string[]
    }) => {
      if (workspaceId === db.diff.workspaceId) {
        if (!paths.length) discarded = null
        else for (const path of paths) discarded?.add(path)
        sendDiff(null)
      }
      return null
    },
  } satisfies HandlerSlice
}
