/**
 * What Esc does once nothing focused has taken it: a slide-out over the page closes first,
 * and only then does a working agent stop. Views register what they have on screen; the
 * order is fixed here, not by which listener happened to be added first.
 */
const LAYERS = ['slideOut', 'agent'] as const

type EscapeLayer = (typeof LAYERS)[number]

/** Returns whether it did something, which ends the chain. */
type EscapeHandler = () => boolean

const handlers = new Map<EscapeLayer, Set<EscapeHandler>>(
  LAYERS.map((layer) => [layer, new Set()]),
)

export function onEscape(layer: EscapeLayer, handle: EscapeHandler) {
  handlers.get(layer)!.add(handle)
  return () => void handlers.get(layer)!.delete(handle)
}

export function runEscape(): boolean {
  for (const layer of LAYERS) {
    for (const handle of handlers.get(layer)!) if (handle()) return true
  }
  return false
}
