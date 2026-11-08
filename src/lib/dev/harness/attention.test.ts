import { describe, expect, it } from 'vitest'
import type { ThreadStatus, WorkspaceLifecycle } from '$lib/ipc/bindings'
import attentionCases from '$lib/ipc/rule-cases/needs-attention.json?raw'
import { needsAttention } from './attention'

/** The cases the core's `workspaces::needs_attention` is tested against. */
const cases = JSON.parse(attentionCases) as {
  lifecycle: WorkspaceLifecycle
  own: ThreadStatus | null
  pending: number
  needsAttention: boolean
}[]

describe('the harness says a workspace needs you as the core does', () => {
  it.each(cases)('$lifecycle, own $own, $pending pending', (row) => {
    expect(needsAttention(row.lifecycle, row.own, row.pending)).toBe(
      row.needsAttention,
    )
  })
})
