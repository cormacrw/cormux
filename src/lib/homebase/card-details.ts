import { plural } from '$lib/sidebar/status'
import { formatRelativeAge } from './relative-time'

export type CardDetailPart =
  | { kind: 'review'; number: number }
  | { kind: 'branch'; name: string }
  | { kind: 'agents'; count: number }
  | { kind: 'files'; count: number }
  | { kind: 'created'; fromMs: number }

export function buildCardDetailParts(input: {
  branch: string
  agentCount: number
  modifiedFiles: number
  createdAtMs: number | null
  reviewPr: number | null
  nowMs: number
}): CardDetailPart[] {
  const parts: CardDetailPart[] = []
  if (input.reviewPr != null) {
    parts.push({ kind: 'review', number: input.reviewPr })
  }
  parts.push({ kind: 'branch', name: input.branch })
  parts.push({ kind: 'agents', count: input.agentCount })
  if (input.modifiedFiles > 0) {
    parts.push({ kind: 'files', count: input.modifiedFiles })
  }
  if (input.createdAtMs != null) {
    parts.push({ kind: 'created', fromMs: input.createdAtMs })
  }
  return parts
}

export function formatCreatedAge(fromMs: number, nowMs: number): string {
  return `Created ${formatRelativeAge(fromMs, nowMs)}`
}

export function formatModifiedFiles(count: number): string {
  return `${plural(count, 'modified file', 'modified files')}`
}
