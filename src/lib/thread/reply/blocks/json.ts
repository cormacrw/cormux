import { defineBlock, fence } from '../block'

/** Shorter JSON reads fine as a code block. */
const JSON_TREE_MIN_LINES = 12

export const jsonBlock = defineBlock({
  kind: 'json',
  match: fence('json'),
  parse: (code): { value: unknown } | null => {
    if (code.split('\n').length < JSON_TREE_MIN_LINES) return null
    try {
      return { value: JSON.parse(code) as unknown }
    } catch {
      return null
    }
  },
})
