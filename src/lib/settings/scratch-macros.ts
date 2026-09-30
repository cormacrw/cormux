/** A named prompt that starts a scratch in the background from the palette. */
export type ScratchMacro = {
  id: string
  name: string
  prompt: string
}

export const SCRATCH_MACROS_KEY = 'scratchMacros'

/** Stored as JSON in one settings row; anything malformed is dropped. */
export function parseScratchMacros(raw: string | undefined): ScratchMacro[] {
  if (!raw) return []
  let parsed: unknown
  try {
    parsed = JSON.parse(raw)
  } catch {
    return []
  }
  if (!Array.isArray(parsed)) return []
  return parsed.flatMap((item) => {
    if (!item || typeof item !== 'object') return []
    const { id, name, prompt } = item as Record<string, unknown>
    if (typeof id !== 'string' || !id) return []
    return [
      {
        id,
        name: typeof name === 'string' ? name : '',
        prompt: typeof prompt === 'string' ? prompt : '',
      },
    ]
  })
}

export function serializeScratchMacros(macros: ScratchMacro[]): string {
  return JSON.stringify(
    macros.map(({ id, name, prompt }) => ({ id, name, prompt })),
  )
}

/** Only macros with a name and a prompt can be run. */
export function isRunnableMacro(macro: ScratchMacro) {
  return macro.name.trim().length > 0 && macro.prompt.trim().length > 0
}

export function newScratchMacroId() {
  return `macro-${Date.now().toString(36)}${Math.random().toString(36).slice(2, 6)}`
}

/** The palette's subtitle: the prompt's first 20 characters on one line. */
export function macroPromptPreview(prompt: string, length = 20) {
  const flat = prompt.trim().replace(/\s+/g, ' ')
  return flat.length > length ? `${flat.slice(0, length).trimEnd()}…` : flat
}

/** The runnable macro whose name is exactly this query, ignoring case. */
export function macroNamed(macros: ScratchMacro[], query: string) {
  const name = query.trim().toLowerCase()
  if (!name) return undefined
  return macros.find(
    (macro) =>
      isRunnableMacro(macro) && macro.name.trim().toLowerCase() === name,
  )
}

/** Text typed after the macro chip goes on the end of the saved prompt. */
export function macroPromptWithExtra(prompt: string, extra: string) {
  const base = prompt.trim()
  const more = extra.trim()
  return more ? `${base}\n\n${more}` : base
}
