import type { RunStep } from './timeline-types'
import { editPath } from './timeline/step-text'
import { plural } from '$lib/format/plural'

function isRole(step: RunStep, kind: string) {
  return step.kind === 'tool' && step.role.kind === kind
}

/** `Read 12 files, edited 3, ran 4 commands`. */
export function toolRunSummary(steps: RunStep[]): string {
  const of = (test: (step: RunStep) => boolean) => steps.filter(test).length
  const reads = of((step) => isRole(step, 'read'))
  const searches = of(
    (step) => isRole(step, 'search') || isRole(step, 'toolLookup'),
  )
  const edits = editedPaths(steps).length
  const commands = of((step) => isRole(step, 'command'))
  const agents = of((step) => isRole(step, 'subagent'))
  const other =
    steps.length -
    reads -
    searches -
    commands -
    agents -
    of((step) => isRole(step, 'edit'))
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

export function editedPaths(steps: RunStep[]): string[] {
  const paths = steps.flatMap((step) =>
    step.kind === 'tool' && step.role.kind === 'edit' && editPath(step)
      ? [editPath(step)]
      : [],
  )
  return [...new Set(paths)]
}
