export type EngineMark = 'CC'

const ENGINE_MARKS: Record<string, EngineMark> = {
  claude: 'CC',
}

const ENGINE_NAMES: Record<string, string> = {
  claude: 'Claude Code',
}

/** Takes an engine id (`claude`) or its display name (`Claude Code`). */
export function engineMark(engine: string): EngineMark {
  const kind =
    Object.keys(ENGINE_NAMES).find((key) => ENGINE_NAMES[key] === engine) ??
    engine
  return ENGINE_MARKS[kind] ?? 'CC'
}

export function engineDisplayName(engine: string): string {
  return ENGINE_NAMES[engine] ?? engine
}
