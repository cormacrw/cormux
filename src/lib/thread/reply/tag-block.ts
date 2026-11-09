/**
 * Machine-readable data an agent appends to a reply as `<name>JSON</name>`. The block that counts
 * is the one the core reads too (src-tauri/src/review/findings_block.rs): the last opening tag,
 * and only once it is closed. Earlier ones are quotes, so they stay in the prose. Both are tested
 * against `$lib/ipc/rule-cases/findings-block.json`.
 */
type TagBlockMatch =
  | { kind: 'block'; start: number; end: number; body: string }
  /** Still being written: everything from `start` on waits until it is closed. */
  | { kind: 'unfinished'; start: number }

/** Agents often wrap JSON in a ```json fence even when asked not to. */
function stripCodeFence(body: string): string {
  if (!body.startsWith('```')) return body
  const rest = body.split('\n').slice(1).join('\n')
  return rest.replace(/```\s*$/, '').trim()
}

export function findTagBlock(
  text: string,
  name: string,
  streaming: boolean,
): TagBlockMatch | null {
  const open = `<${name}>`
  const close = `</${name}>`
  const start = text.lastIndexOf(open)
  if (start < 0) {
    if (!streaming) return null
    for (let length = open.length - 1; length > 0; length--) {
      if (text.endsWith(open.slice(0, length))) {
        return { kind: 'unfinished', start: text.length - length }
      }
    }
    return null
  }
  const bodyStart = start + open.length
  const closeAt = text.indexOf(close, bodyStart)
  if (closeAt < 0) return streaming ? { kind: 'unfinished', start } : null
  return {
    kind: 'block',
    start,
    end: closeAt + close.length,
    body: stripCodeFence(text.slice(bodyStart, closeAt).trim()),
  }
}
