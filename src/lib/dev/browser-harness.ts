import { clickupHandlers } from './harness/handlers/clickup'
import { coreHandlers } from './harness/handlers/core'
import { diffHandlers } from './harness/handlers/diffs'
import { gitHandlers, gitUnsupported } from './harness/handlers/git'
import { repoHandlers, repoUnsupported } from './harness/handlers/repos'
import { threadHandlers, threadUnsupported } from './harness/handlers/threads'
import {
  workspaceHandlers,
  workspaceUnsupported,
} from './harness/handlers/workspaces'
import { pluginHandlers } from './harness/plugins'
import { createRuntime, type HarnessRuntime } from './harness/runtime'
import { readScenario } from './harness/scenario'
import type { HandlerTable } from './harness/types'

type InvokeArgs = Record<string, unknown>
type InvokeHandler = (args: InvokeArgs) => unknown

const UNSUPPORTED = [
  ...gitUnsupported,
  ...repoUnsupported,
  ...threadUnsupported,
  ...workspaceUnsupported,
]

function isRealTauri(): boolean {
  const internals = (
    window as Window & {
      __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label: string } } }
    }
  ).__TAURI_INTERNALS__
  return internals?.metadata?.currentWindow?.label != null
}

/** Every command in `bindings.ts`; `satisfies` is what fails `pnpm check` on a missing one. */
function commandHandlers(runtime: HarnessRuntime) {
  return {
    ...clickupHandlers(runtime),
    ...coreHandlers(runtime),
    ...diffHandlers(runtime),
    ...gitHandlers(runtime),
    ...repoHandlers(runtime),
    ...threadHandlers(runtime),
    ...workspaceHandlers(runtime),
  } satisfies HandlerTable<(typeof UNSUPPORTED)[number]>
}

function createInvoke(runtime: HarnessRuntime) {
  const handlers = {
    ...commandHandlers(runtime),
    ...pluginHandlers(runtime),
  } as unknown as Record<string, InvokeHandler | undefined>
  const slow: Partial<Record<string, number>> = runtime.scenario.slow ?? {}
  const failing: Partial<Record<string, string>> =
    runtime.scenario.failing ?? {}

  return async (cmd: string, args: InvokeArgs = {}) => {
    const failure = failing[cmd]
    // A rejection that isn't an `Error`, as Tauri delivers a command's `Err`.
    if (failure !== undefined) throw { kind: 'Workspace', message: failure }
    const handler = handlers[cmd]
    if (handler) {
      const ms = slow[cmd]
      if (ms) await new Promise((resolve) => setTimeout(resolve, ms))
      return handler(args)
    }
    if (cmd.startsWith('plugin:')) return null
    throw new Error(
      (UNSUPPORTED as string[]).includes(cmd)
        ? `Browser harness: ${cmd} is unsupported. Give it a handler in src/lib/dev/harness/handlers if this flow needs it.`
        : `Browser harness: unknown command ${cmd}.`,
    )
  }
}

/**
 * Stubs Tauri IPC so `pnpm dev` and Playwright can drive the UI in Chromium.
 * No-ops when the real webview already injected internals.
 */
export function installBrowserHarness() {
  if (typeof window === 'undefined' || isRealTauri()) return
  const runtime = createRuntime(readScenario())
  const invoke = createInvoke(runtime)
  window.__HARNESS__ = runtime.controls

  void import('$lib/feedback/show-toast').then(({ showToast }) => {
    runtime.controls.showToast = showToast
  })

  Object.assign(window, {
    __TAURI_INTERNALS__: {
      invoke,
      transformCallback: runtime.transformCallback,
      unregisterCallback: runtime.unregisterCallback,
      convertFileSrc(filePath: string) {
        return filePath
      },
      metadata: {
        currentWindow: { label: 'main' },
        currentWebview: { label: 'main' },
      },
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener: runtime.unlisten,
    },
  })

  document.documentElement.dataset.cormuxHarness = 'browser'
}
