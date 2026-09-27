export type PullRequest = {
  id: string
  title: string
  repo: string
  number: number
}

export class PrsStore {
  items = $state<PullRequest[]>([])

  readonly count = $derived(this.items.length)

  hydrate(items: PullRequest[]) {
    this.items = items
  }
}

export const prs = new PrsStore()
