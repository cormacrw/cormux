import type { PaletteCommand } from './types'

/** Close palette first, then run on the next tick (spec §02). */
export function schedulePaletteCommand(
  close: () => void,
  command: PaletteCommand,
) {
  close()
  queueMicrotask(() => {
    command.run()
  })
}
