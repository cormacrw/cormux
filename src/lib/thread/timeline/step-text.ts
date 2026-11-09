import type { RunStep, ToolStep } from '../timeline-types'

/** What a step says: its title, a short detail beside it, and the full detail behind it. */
type StepText = {
  title: string
  detail?: string
  full?: string
}

const COMMAND_PREVIEW_CHARS = 50

/** `https://www.example.com/a/b` → `example.com/a/b`; the full URL stays in the tooltip. */
function shortUrl(url: string | null): string | undefined {
  if (!url) return undefined
  return url
    .replace(/^[a-z]+:\/\//i, '')
    .replace(/^www\./, '')
    .replace(/\/$/, '')
}

function toolText(step: ToolStep): StepText {
  const { role } = step
  const detail = step.detail ?? undefined
  switch (role.kind) {
    case 'edit':
      return { title: role.verb, detail: editPath(step) || undefined }
    case 'subagent':
      return { title: 'Agent', detail }
    case 'command': {
      const { command } = role
      const preview =
        command.length > COMMAND_PREVIEW_CHARS
          ? `${command.slice(0, COMMAND_PREVIEW_CHARS).trimEnd()}…`
          : command
      return {
        title: command ? 'Ran' : 'Ran a command',
        detail: preview || undefined,
        full: command || undefined,
      }
    }
    case 'fetch':
      return { title: 'Fetched', detail: shortUrl(step.detail), full: detail }
    case 'webSearch':
      return { title: 'Searched the web', detail, full: detail }
    case 'skill':
      return { title: 'Used skill', detail, full: detail }
    case 'planMode':
      return { title: 'Switched to plan mode', full: detail }
    case 'toolLookup':
      return { title: 'Looked up tools', detail, full: detail }
    case 'mcp':
      return {
        title: `Used ${role.server}`,
        detail: detail ? `${role.action} · ${detail}` : role.action,
        full: detail,
      }
    case 'read':
    case 'search':
    case 'other':
      return {
        title: step.title,
        detail: (step.detail ?? step.locations.join(', ')) || undefined,
        full: detail,
      }
  }
}

/** The file an edit touched; empty when the engine didn't say. */
export function editPath(step: ToolStep): string {
  return step.locations[0] ?? ''
}

export function stepText(step: RunStep): StepText {
  if (step.kind === 'tool') return toolText(step)
  return {
    title: step.title,
    detail: step.detail || undefined,
    full: step.detail ?? undefined,
  }
}

export function basename(path: string): string {
  return path.split('/').pop() ?? path
}

/** One line for a step, as a subagent's latest move reads: `Read src/a.ts`, `Edited a.ts`. */
export function stepLine(step: RunStep): string {
  if (step.kind === 'tool' && step.role.kind === 'edit') {
    return `${step.role.verb} ${basename(editPath(step))}`
  }
  const { title, detail } = stepText(step)
  return [title, detail].filter(Boolean).join(' ')
}

/** Under a minute: `Worked for 12s`. Otherwise `Worked for 2m 5s`. */
export function workedForLabel(startMs: number, endMs: number): string {
  const elapsed = Math.max(0, endMs - startMs)
  const total = elapsed === 0 ? 0 : Math.max(1, Math.round(elapsed / 1000))
  const minutes = Math.floor(total / 60)
  const seconds = total % 60
  if (minutes === 0) return `Worked for ${seconds}s`
  return `Worked for ${minutes}m ${seconds}s`
}
