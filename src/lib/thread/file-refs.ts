// Files an agent names in a reply, like `src/lib/thread/typing.ts:40`.

export type FileRef = {
  /** Where in the text it starts, and how long it is. */
  index: number
  length: number
  path: string
  line: number | null
}

const NAME = String.raw`[\w@.-]+`
// A path has a folder or a line number; `node.js` or `e.g.` alone is just a word.
const REF = new RegExp(
  String.raw`(?<![\w/.:@-])((?:\.{0,2}/)?(?:${NAME}/)+${NAME}\.[a-zA-Z0-9]{1,8}|${NAME}\.[a-zA-Z][a-zA-Z0-9]{0,7}(?=:\d))(?::(\d+))?(?![\w/-])`,
  'g',
)

export function findFileRefs(text: string): FileRef[] {
  return Array.from(text.matchAll(REF), (match) => ({
    index: match.index,
    length: match[0].length,
    path: match[1]!,
    line: match[2] ? Number(match[2]) : null,
  }))
}

/** The ref when all of `text` is one, as inline code usually is. */
export function wholeFileRef(text: string): FileRef | null {
  const trimmed = text.trim()
  const [ref] = findFileRefs(trimmed)
  return ref?.length === trimmed.length ? ref : null
}

/** A reference relative to the worktree, which is how Changes knows its files. */
export function worktreePath(
  path: string,
  workdir: string | undefined,
): string {
  const relative = path.replace(/^\.\//, '')
  if (!workdir) return relative
  const root = workdir.endsWith('/') ? workdir : `${workdir}/`
  return relative.startsWith(root) ? relative.slice(root.length) : relative
}
