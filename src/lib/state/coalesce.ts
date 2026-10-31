/**
 * Runs `work` once however many times it's asked for while running, then once more if anything
 * asked during it, so the last caller's request always sees fresh data.
 */
export function coalesce(work: () => Promise<void>): () => Promise<void> {
  let running: Promise<void> | null = null
  let again = false

  async function loop() {
    do {
      again = false
      await work()
    } while (again)
  }

  return () => {
    if (running) {
      again = true
      return running
    }
    running = loop().finally(() => {
      running = null
    })
    return running
  }
}
