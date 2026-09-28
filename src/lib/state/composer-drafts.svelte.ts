import { applyDraftUpdate, draftText } from '$lib/composer/drafts'

export class ComposerDraftsStore {
  byThread = $state<Record<string, string>>({})

  textFor(threadId: string): string {
    return draftText(this.byThread, threadId)
  }

  setFor(threadId: string, text: string) {
    this.byThread = applyDraftUpdate(this.byThread, threadId, text)
  }
}

export const composerDrafts = new ComposerDraftsStore()
