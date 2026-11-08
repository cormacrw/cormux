import type { MemorySample } from '$lib/ipc/bindings'
import { formatMemoryGb } from '$lib/format/memory'
import { onSnapshot } from '$lib/core/sync'

export class MemoryStore {
  totalBytes = $state(0)

  readonly label = $derived(formatMemoryGb(this.totalBytes))

  hydrate(sample: MemorySample | null | undefined) {
    this.totalBytes = sample?.totalBytes ?? 0
  }
}

export const memory = new MemoryStore()

onSnapshot((snapshot) => memory.hydrate(snapshot.memory))
