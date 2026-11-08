import type { Token } from 'marked'

/**
 * Where a block sits in a reply: a top-level markdown token (a fenced block, display math), or
 * a `<name>…</name>` tag the agent appends as data (see `tag-block.ts`).
 */
export type BlockMatch = TokenMatch | { tag: string }

/** The block's body when `token` is one, else null. */
type TokenMatch = { token: (token: Token) => string | null }

/** A reply block that renders as a component. `parse` returning null leaves it as prose. */
export type BlockDef<K extends string, D> = {
  kind: K
  match: BlockMatch
  parse: (body: string) => D | null
  /** What ⌘F finds in a tag block; inline blocks are searched by their source. */
  text?: (data: D) => string
}

/** A tag block Cormux reads elsewhere, so the conversation never shows it. */
export type HiddenBlockDef<K extends string> = {
  kind: K
  match: { tag: string }
  hidden: true
}

export function defineBlock<K extends string, D>(
  def: BlockDef<K, D>,
): BlockDef<K, D> {
  return def
}

/** A fenced code block in one of `languages` (`''` is a fence without one). */
export function fence(...languages: string[]): TokenMatch {
  return {
    token: (token) => {
      if (token.type !== 'code') return null
      const language =
        String(token.lang ?? '')
          .split(/\s/)[0]
          ?.toLowerCase() ?? ''
      return languages.includes(language) ? String(token.text) : null
    },
  }
}

export function tag(name: string): { tag: string } {
  return { tag: name }
}
