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

/** Takes an engine id (`cursor`) or its display name (`Cursor`). */
export function engineMark(engine: string): EngineMark {
  const kind =
    Object.keys(ENGINE_NAMES).find((key) => ENGINE_NAMES[key] === engine) ??
    engine
  return ENGINE_MARKS[kind] ?? 'CC'
}

export function engineDisplayName(engine: string): string {
  return ENGINE_NAMES[engine] ?? engine
}
