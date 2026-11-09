import { FINDINGS_BLOCK_TAG } from '$lib/ipc/bindings'
import { tag, type HiddenBlockDef } from '../block'

/**
 * The Reviewer ends its reply with its findings as JSON in this block. The core reads them into
 * the Findings tab (src-tauri/src/review/findings_block.rs), so the conversation never shows it.
 */
export const findingsBlock: HiddenBlockDef<'findings'> = {
  kind: 'findings',
  match: tag(FINDINGS_BLOCK_TAG),
  hidden: true,
}
