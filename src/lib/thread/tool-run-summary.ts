import type { ToolRunStep } from './timeline-types'
import { plural } from '$lib/format/plural'

/** A run this long folds down to its summary once the agent has moved on. */
export const COLLAPSE_RUN_AT = 6

/** `Read 12 files, edited 3, ran 4 commands`. */
export function toolRunSummary(steps: ToolRunStep[]): string {
  const of = (test: (step: ToolRunStep) => boolean) => steps.filter(test).length
  const reads = of((step) => step.kind === 'tool' && step.icon === 'file')
  const searches = of((step) => step.kind === 'tool' && step.icon === 'search')
  const edits = editedPaths(steps).length
  const commands = of(
    (step) => step.kind === 'tool' && step.title.startsWith('Ran'),
  )
  const agents = of((step) => step.kind === 'tool' && Boolean(step.agent))
  const other =
    steps.length -
    reads -
    searches -
    commands -
    agents -
    of((step) => step.kind === 'edit')
  const parts = [
    reads ? `read ${plural(reads, 'file')}` : '',
    searches ? `ran ${plural(searches, 'search', 'searches')}` : '',
    edits ? `edited ${plural(edits, 'file')}` : '',
    commands ? `ran ${plural(commands, 'command')}` : '',
    agents ? `ran ${plural(agents, 'agent')}` : '',
    other > 0 ? plural(other, 'other step') : '',
  ].filter(Boolean)
  const line = parts.join(', ')
  return line.charAt(0).toUpperCase() + line.slice(1)
}

/** Each file the run changed, once. */
export function editedPaths(steps: ToolRunStep[]): string[] {
  const paths = steps.flatMap((step) =>
    step.kind === 'edit' && step.path ? [step.path] : [],
  )
  return [...new Set(paths)]
}
