import type { DiffFile } from '$lib/ipc/bindings'

const langByExt: Record<string, string> = {
  ts: 'typescript',
  tsx: 'typescript',
  js: 'javascript',
  jsx: 'javascript',
  mjs: 'javascript',
  svelte: 'xml',
  html: 'xml',
  rs: 'rust',
  py: 'python',
  go: 'go',
  json: 'json',
  md: 'markdown',
  css: 'css',
  yml: 'yaml',
  yaml: 'yaml',
  toml: 'ini',
  sh: 'bash',
  sql: 'sql',
}

/** highlight.js language for a path; git-diff-view highlights with lowlight. */
export function languageForPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  return langByExt[ext] ?? 'plaintext'
}

/** git-diff-view wants the `---`/`+++` header the backend strips; without it no lines parse. */
export function toDiffViewData(file: DiffFile) {
  const lang = languageForPath(file.path)
  const hunks = file.hunks.map((hunk) => `${hunk.header}\n${hunk.body}`).join('')
  return {
    oldFile: { fileName: file.path, fileLang: lang },
    newFile: { fileName: file.path, fileLang: lang },
    hunks: [`--- a/${file.path}\n+++ b/${file.path}\n${hunks}`],
  }
}
