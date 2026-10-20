import type { ComposerAttachment } from '$lib/composer/attachments'
import { applyDraftUpdate, draftText } from '$lib/composer/drafts'

export class ComposerDraftsStore {
  byThread = $state<Record<string, string>>({})
  /** Images and files waiting to go with the next message, per thread. */
  attachmentsByThread = $state<Record<string, ComposerAttachment[]>>({})

  attachmentsFor(threadId: string): ComposerAttachment[] {
    return this.attachmentsByThread[threadId] ?? []
  }

  setAttachments(threadId: string, attachments: ComposerAttachment[]) {
    this.attachmentsByThread = {
      ...this.attachmentsByThread,
      [threadId]: attachments,
    }
  }

  addAttachments(threadId: string, attachments: ComposerAttachment[]) {
    this.setAttachments(threadId, [
      ...this.attachmentsFor(threadId),
      ...attachments,
    ])
  }

  /** A thumbnail that arrived after the attachment did (images attached by path). */
  setPreview(threadId: string, id: string, previewUrl: string) {
    this.setAttachments(
      threadId,
      this.attachmentsFor(threadId).map((attachment) =>
        attachment.id === id ? { ...attachment, previewUrl } : attachment,
      ),
    )
  }

  removeAttachment(threadId: string, id: string) {
    this.setAttachments(
      threadId,
      this.attachmentsFor(threadId).filter(
        (attachment) => attachment.id !== id,
      ),
    )
  }

  textFor(threadId: string): string {
    return draftText(this.byThread, threadId)
  }

  setFor(threadId: string, text: string) {
    this.byThread = applyDraftUpdate(this.byThread, threadId, text)
  }
}

export const composerDrafts = new ComposerDraftsStore()
