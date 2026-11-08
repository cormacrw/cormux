import type { HarnessRuntime } from './runtime'

type PluginHandler = (args: Record<string, unknown>) => unknown

/** Tauri's own plugin commands, which `bindings.ts` doesn't know. Any other plugin call is a no-op. */
export function pluginHandlers(
  runtime: HarnessRuntime,
): Record<string, PluginHandler> {
  return {
    'plugin:event|listen': (args) =>
      runtime.listen(String(args.event), Number(args.handler)),
    // `unregisterListener` already dropped it.
    'plugin:event|unlisten': () => null,
    'plugin:window|is_focused': () => true,
    'plugin:window|is_fullscreen': () => runtime.scenario.fullscreen === true,
    'plugin:window|set_title': (args) => {
      if (typeof args.value === 'string') document.title = args.value
      return null
    },
    'plugin:opener|open_url': (args) => {
      runtime.controls.opened.push(String(args.url))
      return null
    },
  }
}
