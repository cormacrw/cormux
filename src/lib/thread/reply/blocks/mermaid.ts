import { defineBlock, fence } from '../block'

export const mermaidBlock = defineBlock({
  kind: 'mermaid',
  match: fence('mermaid'),
  parse: (code) => code,
})
