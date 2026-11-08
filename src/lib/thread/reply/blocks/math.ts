import { defineBlock, fence } from '../block'

const latexFence = fence('math', 'latex')
const DISPLAY_MATH = /^\s*\$\$([\s\S]+)\$\$\s*$/

/** A `math` or `latex` fence, or a paragraph that is all `$$…$$`. */
export const mathBlock = defineBlock({
  kind: 'math',
  match: {
    token: (token) => {
      const fenced = latexFence.token(token)
      if (fenced !== null) return fenced
      if (token.type !== 'paragraph') return null
      return DISPLAY_MATH.exec(token.raw)?.[1]?.trim() ?? null
    },
  },
  parse: (tex) => tex,
})
