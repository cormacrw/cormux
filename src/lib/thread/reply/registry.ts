import { chartBlock } from './blocks/chart'
import { diffBlock } from './blocks/diff'
import { findingsBlock } from './blocks/findings'
import { jsonBlock } from './blocks/json'
import { mathBlock } from './blocks/math'
import { mermaidBlock } from './blocks/mermaid'
import { runBlock } from './blocks/run'
import { tasksBlock } from './blocks/tasks'

/**
 * Every block a reply can hold. A new one is a parser here plus its component in the UI's
 * `BLOCK_COMPONENTS`, which the types require. Tag blocks are taken out in this order.
 */
export const BLOCKS = [
  diffBlock,
  mermaidBlock,
  chartBlock,
  jsonBlock,
  runBlock,
  mathBlock,
  findingsBlock,
  tasksBlock,
] as const

type VisibleDef = Exclude<(typeof BLOCKS)[number], { hidden: true }>

type ParsedBlock<Def> = Def extends {
  kind: infer Kind
  parse: (body: string) => infer Data
}
  ? { kind: Kind; data: NonNullable<Data> }
  : never

/** A parsed block that renders as a component. */
export type ReplyBlock = ParsedBlock<VisibleDef>

export type BlockKind = ReplyBlock['kind']
