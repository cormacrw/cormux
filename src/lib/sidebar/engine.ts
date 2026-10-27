export type EngineMark = 'CC' | 'CX' | 'GM'

const ENGINE_MARKS: Record<string, EngineMark> = {
  claude: 'CC',
  codex: 'CX',
  gemini: 'GM',
}

const ENGINE_NAMES: Record<string, string> = {
  claude: 'Claude Code',
  codex: 'Codex',
  gemini: 'Gemini',
}

/** Takes an engine id (`codex`) or its display name (`Codex`). */
export function engineMark(engine: string): EngineMark {
  const kind =
    Object.keys(ENGINE_NAMES).find((key) => ENGINE_NAMES[key] === engine) ??
    engine
  return ENGINE_MARKS[kind] ?? 'CC'
}

export function engineDisplayName(engine: string): string {
  return ENGINE_NAMES[engine] ?? engine
}
