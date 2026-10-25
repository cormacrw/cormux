/**
 * Bangs: a short name like `!test` that stands for a saved prompt. Typed in any
 * prompt bar it becomes the prompt; typed in the command palette it starts a
 * scratch with it. Managed in Settings › Bangs.
 */
export type Bang = {
  id: string
  /** Without the `!`: lowercase letters, digits, `-` and `_`. */
  name: string
  prompt: string
}

export const BANGS_KEY = 'bangs'
/** Scratch macros, which bangs replace; read once to carry them over. */
export const LEGACY_MACROS_KEY = 'scratchMacros'

/** A name as a bang: no `!`, no spaces (they become `-`), lowercase. */
export function bangName(raw: string): string {
  return raw
    .trim()
    .replace(/^!+/, '')
    .toLowerCase()
    .replace(/\s+/g, '-')
    .replace(/[^a-z0-9_-]/g, '')
}

function parseList(raw: string | undefined): Bang[] | null {
  if (!raw) return null
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch {
    return null
  }
  if (!Array.isArray(parsed)) return null
  return parsed.flatMap((item) => {
    if (!item || typeof item !== 'object') return []
    const { id, name, prompt } = item as Record<string, unknown>
    if (typeof id !== 'string' || !id) return []
    return [
      {
        id,
        name: typeof name === 'string' ? bangName(name) : '',
        prompt: typeof prompt === 'string' ? prompt : '',
      },
    ]
  })
}

/**
 * Bangs from their settings row, or, before any are saved, the old scratch
 * macros with their names made into bangs (`Daily standup` → `daily-standup`).
 */
export function parseBangs(
  raw: string | undefined,
  legacyMacros?: string,
): Bang[] {
  return parseList(raw) ?? parseList(legacyMacros) ?? []
}

export function serializeBangs(bangs: Bang[]): string {
  return JSON.stringify(
    bangs.map(({ id, name, prompt }) => ({ id, name, prompt })),
  )
}

/** Only bangs with a name and a prompt can be used. */
export function isUsableBang(bang: Bang) {
  return bang.name.length > 0 && bang.prompt.trim().length > 0
}

export function newBangId() {
  return `bang-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`
}

/** The usable bang with exactly this name (with or without its `!`). */
export function bangNamed(bangs: Bang[], name: string): Bang | undefined {
  const wanted = bangName(name)
  if (!wanted) return undefined
  return bangs.find((bang) => isUsableBang(bang) && bang.name === wanted)
}

/** Usable bangs whose name starts with the query, then those containing it. */
export function filterBangs(bangs: Bang[], query: string): Bang[] {
  const needle = query.toLowerCase()
  const usable = bangs.filter(isUsableBang)
  const starts = usable.filter((bang) => bang.name.startsWith(needle))
  const contains = usable.filter(
    (bang) => !starts.includes(bang) && bang.name.includes(needle),
  )
  return [...starts, ...contains]
}

/** The prompt's start on one line, for lists. */
export function bangPreview(prompt: string, length = 48) {
  const flat = prompt.trim().replace(/\s+/g, ' ')
  return flat.length > length ? `${flat.slice(0, length).trimEnd()}…` : flat
}

export type BangToken = { start: number; end: number; query: string }

/** The `!word` the caret is in, if it starts the draft or follows a space. */
export function bangToken(draft: string, caret: number): BangToken | null {
  const match = /(^|\s)!([\w-]*)$/.exec(draft.slice(0, caret))
  if (!match) return null
  const rest = /^\S*/.exec(draft.slice(caret))?.[0] ?? ''
  if (!/^[\w-]*$/.test(rest)) return null
  return {
    start: match.index + (match[1]?.length ?? 0),
    end: caret + rest.length,
    query: match[2] ?? '',
  }
}

/** The draft with the `!word` replaced by the bang's prompt, and the caret after it. */
export function insertBang(
  draft: string,
  token: BangToken,
  bang: Bang,
): { text: string; caret: number } {
  const prompt = bang.prompt.trim()
  const rest = draft.slice(token.end)
  const gap = rest === '' || /^\s/.test(rest) ? '' : ' '
  return {
    text: draft.slice(0, token.start) + prompt + gap + rest,
    caret: token.start + prompt.length + gap.length,
  }
}

/** On send: any whole `!name` left in a message becomes its bang's prompt. */
export function expandBangs(text: string, bangs: Bang[]): string {
  return text.replace(
    /(^|\s)!([\w-]+)(?=\s|$)/g,
    (whole, lead: string, name: string) => {
      const bang = bangNamed(bangs, name)
      return bang ? `${lead}${bang.prompt.trim()}` : whole
    },
  )
}

/**
 * The palette's `!name more words`: the bang to start a scratch with, and the
 * words after it, added to the end of its prompt. Null when it isn't a bang.
 */
export function paletteBang(
  query: string,
  bangs: Bang[],
): { bang: Bang; extra: string } | null {
  const match = /^\s*!([\w-]+)(?:\s+([\s\S]*))?$/.exec(query)
  if (!match) return null
  const bang = bangNamed(bangs, match[1]!)
  return bang ? { bang, extra: (match[2] ?? '').trim() } : null
}

/** A bang's prompt with what was typed after it in the palette. */
export function bangPromptWithExtra(prompt: string, extra: string) {
  const base = prompt.trim()
  const more = extra.trim()
  return more ? `${base}\n\n${more}` : base
}
