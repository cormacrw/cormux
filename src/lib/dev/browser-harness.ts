import { clickupHandlers } from './harness/handlers/clickup'
import { coreHandlers } from './harness/handlers/core'
import { diffHandlers } from './harness/handlers/diffs'
import { gitHandlers } from './harness/handlers/git'
import { repoHandlers } from './harness/handlers/repos'
import { threadHandlers } from './harness/handlers/threads'
import { workspaceHandlers } from './harness/handlers/workspaces'
import { pluginHandlers } from './harness/plugins'
import { createRuntime, type HarnessRuntime } from './harness/runtime'
import { readScenario } from './harness/scenario'
import { seedFixtures } from './harness/seed'
import type { HandlerTable } from './harness/types'
import { isUnsupported } from './harness/unsupported'

type InvokeArgs = Record<string, unknown>
type InvokeHandler = (args: InvokeArgs) => unknown

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
  } satisfies HandlerTable
}

function createInvoke(runtime: HarnessRuntime) {
  const handlers = {
    ...commandHandlers(runtime),
    ...pluginHandlers(runtime),
  } as unknown as Record<string, InvokeHandler | undefined>

  return async (cmd: string, args: InvokeArgs = {}) => {
    const handler = handlers[cmd]
    if (handler) return handler(args)
    if (cmd.startsWith('plugin:')) return null
    throw new Error(
      isUnsupported(cmd)
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
  const scenario = readScenario()
  seedFixtures(scenario)
  const runtime = createRuntime(scenario)
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
      unregisterListener() {},
    },
  })

  document.documentElement.dataset.cormuxHarness = 'browser'
}
