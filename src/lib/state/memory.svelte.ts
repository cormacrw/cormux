import type { MemorySample } from '$lib/ipc/bindings'
import { memoryBarScale, formatMemoryGb } from '$lib/sidebar/status'

export class MemoryStore {
  totalBytes = $state(0)

  readonly label = $derived(formatMemoryGb(this.totalBytes))
  readonly barScale = $derived(memoryBarScale(this.totalBytes))

  hydrate(sample: MemorySample | null | undefined) {
    this.totalBytes = sample?.totalBytes ?? 0
  }
}

export const memory = new MemoryStore()
