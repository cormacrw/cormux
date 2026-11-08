import { events, type MemorySample } from '$lib/ipc/bindings'
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

// The snapshot has a sample from the start; the core's samples keep it current after that,
// for as long as the app runs.
onSnapshot((snapshot) => memory.hydrate(snapshot.memory))
void events.memorySampled.listen(({ payload }) =>
  memory.hydrate(payload.sample),
)
