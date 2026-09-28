import { fixtureEngines, fixtureSnapshot } from './fixture-snapshot'

type InvokeArgs = Record<string, unknown> | undefined

function isRealTauri(): boolean {
  const internals = (
    window as Window & {
      __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label: string } } }
    }
  ).__TAURI_INTERNALS__
  return internals?.metadata?.currentWindow?.label != null
}

/**
 * Stubs Tauri IPC so `pnpm dev` and Playwright can drive the UI in Chromium.
 * No-ops when the real webview already injected internals.
 */
export function installBrowserHarness() {
  if (typeof window === 'undefined' || isRealTauri()) return

  const callbacks = new Map<number, (...args: unknown[]) => void>()
  let nextCallbackId = 1
  let nextEventId = 1

  const invoke = async (cmd: string, args: InvokeArgs = {}) => {
    if (cmd === 'get_snapshot') return fixtureSnapshot
    if (cmd === 'detect_engines') return fixtureEngines
    if (cmd === 'list_repo_branches') {
      return { branches: ['main', 'develop', 'feat/oauth-login'] }
    }
    if (cmd === 'get_metrics') return fixtureSnapshot.memory
    if (cmd === 'set_setting') return null
    if (cmd === 'plugin:event|listen') return nextEventId++
    if (cmd === 'plugin:event|unlisten') return null
    if (cmd === 'plugin:window|is_focused') return true
    if (cmd === 'plugin:window|set_title') {
      const title = args.value
      if (typeof title === 'string') document.title = title
      return null
    }
    if (cmd.startsWith('plugin:window|')) return null
    if (cmd.startsWith('plugin:')) return null
    return null
  }

  Object.assign(window, {
    __TAURI_INTERNALS__: {
      invoke,
      transformCallback(callback?: (...args: unknown[]) => void, once = false) {
        const id = nextCallbackId++
        callbacks.set(id, (...cbArgs: unknown[]) => {
          if (once) callbacks.delete(id)
          callback?.(...cbArgs)
        })
        return id
      },
      unregisterCallback(id: number) {
        callbacks.delete(id)
      },
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
