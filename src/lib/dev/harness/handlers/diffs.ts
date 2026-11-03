import type { DiffTarget } from '$lib/ipc/bindings'
import { fixtureDiff } from '../../fixture-snapshot'
import type { HarnessRuntime } from '../runtime'
import type { HandlerSlice } from '../types'

export function diffHandlers(runtime: HarnessRuntime) {
  const { scenario } = runtime

  const diffFiles = () => {
    const count = scenario.bigDiff ?? 0
    if (!count) return fixtureDiff.files
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
    const diff = {
      ...fixtureDiff,
      target,
      files: target ? diffFiles() : uncommittedFiles(),
    }
    runtime.send(runtime.diffChannel, {
      workspaceId: fixtureDiff.workspaceId,
      path: '',
      diff,
    })
  }

  const holdIfSlow = async () => {
    if (scenario.slowDiff) {
      await new Promise((resolve) => setTimeout(resolve, 1500))
    }
  }

  return {
    subscribe_diffs: ({
      workspaceId,
      channel,
    }: {
      workspaceId: string
      channel: { id: number }
    }) => {
      if (workspaceId === fixtureDiff.workspaceId) {
        runtime.diffChannel = { id: channel.id, index: 0 }
        queueMicrotask(() => sendDiff(null))
      }
      return runtime.newId()
    },
    refresh_workspace_diff: async () => {
      await holdIfSlow()
      return null
    },
    set_workspace_diff_target: async ({
      workspaceId,
      target,
    }: {
      workspaceId: string
      target: DiffTarget | null
    }) => {
      await holdIfSlow()
      if (workspaceId === fixtureDiff.workspaceId) {
        setTimeout(() => sendDiff(target ?? null), 100)
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
      if (workspaceId === fixtureDiff.workspaceId) {
        if (!paths.length) discarded = null
        else for (const path of paths) discarded?.add(path)
        sendDiff(null)
      }
      return null
    },
  } satisfies HandlerSlice
}
