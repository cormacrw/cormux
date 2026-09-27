export type EngineMark = 'CC' | 'CU' | 'CX' | 'GM'

const ENGINE_MARKS: Record<string, EngineMark> = {
  claude: 'CC',
  cursor: 'CU',
  codex: 'CX',
  gemini: 'GM',
}

const ENGINE_NAMES: Record<string, string> = {
  claude: 'Claude Code',
  cursor: 'Cursor',
  codex: 'Codex',
  gemini: 'Gemini',
}

export function engineMark(engine: string): EngineMark {
  return ENGINE_MARKS[engine] ?? 'CC'
}

export function engineDisplayName(engine: string): string {
  return ENGINE_NAMES[engine] ?? engine
}
