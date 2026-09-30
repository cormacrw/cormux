/** Mirrors `OPEN_TAG` / `CLOSE_TAG` in src-tauri/src/findings_block.rs. */
const OPEN_TAG = '<cormux-findings>'
const CLOSE_TAG = '</cormux-findings>'

/**
 * The Reviewer ends its reply with its findings as JSON inside these tags. Cormux
 * reads them into the Findings tab, so the conversation hides the block, including
 * a half-streamed opening tag at the end of the text.
 */
export function hideFindingsBlock(text: string): string {
  const start = text.indexOf(OPEN_TAG)
  if (start >= 0) {
    const close = text.indexOf(CLOSE_TAG, start)
    const after = close >= 0 ? text.slice(close + CLOSE_TAG.length) : ''
    return (text.slice(0, start) + after).trimEnd()
  }
  for (let length = OPEN_TAG.length - 1; length > 0; length--) {
    if (text.endsWith(OPEN_TAG.slice(0, length))) {
      return text.slice(0, -length).trimEnd()
    }
  }
  return text
}
