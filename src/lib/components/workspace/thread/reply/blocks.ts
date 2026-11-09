import type { Component } from 'svelte'
import type { BlockKind, ReplyBlock } from '$lib/thread/reply/registry'
import ChartBlock from './ChartBlock.svelte'
import DiffBlock from './DiffBlock.svelte'
import JsonBlock from './JsonBlock.svelte'
import MathBlock from './MathBlock.svelte'
import MermaidBlock from './MermaidBlock.svelte'
import RunSummaryCard from './RunSummaryCard.svelte'
import TaskDraftCards from './TaskDraftCards.svelte'

/** ReplyMarkdown wraps each block in `data-reply-block`. */
type BlockProps<K extends BlockKind> = {
  data: Extract<ReplyBlock, { kind: K }>['data']
  /** The thread's worktree, for blocks that point at its files. */
  workdir?: string
  /** Opens a file in Changes; without it, a block's files aren't links. */
  revealFile?: (path: string, line?: number | null) => void
}

export const BLOCK_COMPONENTS: {
  [K in BlockKind]: Component<BlockProps<K>>
} = {
  diff: DiffBlock,
  mermaid: MermaidBlock,
  chart: ChartBlock,
  json: JsonBlock,
  run: RunSummaryCard,
  math: MathBlock,
  tasks: TaskDraftCards,
}
