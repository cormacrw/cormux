import { hideFindingsBlock } from '$lib/findings/findings-block'
import { splitTaskCards } from '$lib/tasks/task-cards-block'
import { memoByText } from './memo'

/** What a reply shows once its machine-readable blocks are taken out; null if that's nothing. */
export const replyParts = memoByText((text: string) => {
  const shown = hideFindingsBlock(text)
  return shown ? splitTaskCards(shown) : null
})
