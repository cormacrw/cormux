import type { TranscriptEvent } from '$lib/ipc/bindings'
import { fixtureSnapshot, fixtureTimeline } from '../fixture-snapshot'
import type { HarnessScenario } from './scenario'

/** A sent message as the core saves it, with any attachments named after it. */
export function shownMessage(text: string, names: string[]): string {
  if (!names.length) return text
  const line = `Attached: ${names.join(', ')}`
  return text ? `${text}\n\n${line}` : line
}

/** The next `seq` in a thread: the store numbers rows at write time. */
export function nextSeq(threadId: string) {
  return (
    Math.max(
      0,
      ...fixtureTimeline
        .filter((row) => row.threadId === threadId)
        .map((row) => row.seq),
    ) + 1
  )
}

function seedLongThread(turns: number) {
  let id = 100_000
  let seq = 0
  const push = (kind: string, event: object) => {
    seq += 1
    id += 1
    fixtureTimeline.push({
      id,
      threadId: 'th-lead',
      seq,
      kind,
      payload: JSON.stringify(event),
      createdAt: '2026-09-28 12:00:00',
    })
  }
  for (let turn = 1; turn <= turns; turn += 1) {
    push('message', {
      type: 'messageChunk',
      role: 'user',
      text: `Question ${turn}`,
    })
    push('message', {
      type: 'messageChunk',
      role: 'agent',
      text: `Answer ${turn}`,
    })
    push('turn_end', { type: 'turnEnd', stop_reason: 'end_turn', error: null })
  }
  // Switching away from a branch and back writes two notes with the same title.
  const switches = ['feat/test', 'feat/colors', 'feat/test']
  switches.forEach((branch, index) => {
    const previous = switches[index - 1] ?? 'main'
    push('tool', {
      type: 'step',
      id: `step-switch-${index}`,
      title: `Switched to \`${branch}\``,
      detail: 'Checked out in this worktree',
      step: {
        kind: 'branchSwitched',
        branch,
        change: { kind: 'checkedOut', previous },
      },
    } satisfies TranscriptEvent)
  })
}

function seedReviewFindings(findings: HarnessScenario['reviewFindings']) {
  const persisted = fixtureSnapshot.persisted
  const row = persisted.workspaces.find(
    (workspace) => workspace.id === 'ws-auth',
  )!
  row.kind = 'review'
  row.prNumber = 2
  fixtureSnapshot.workspaces.find(
    (workspace) => workspace.id === 'ws-auth',
  )!.kind = 'review'
  Object.assign(
    persisted.threads.find((thread) => thread.id === 'th-lead')!,
    { title: 'Reviewer', role: 'reviewer' },
  )
  persisted.findings = findings!
  persisted.settings = [
    ...persisted.settings,
    { key: 'review:ws-auth/status', value: 'ready' },
  ]
}

function seedLeadUsage(usage: NonNullable<HarnessScenario['leadUsage']>) {
  Object.assign(
    fixtureSnapshot.persisted.threads.find(
      (thread) => thread.id === 'th-lead',
    )!,
    usage,
  )
  // As a real turn does, log the usage it reported, stamped now.
  fixtureTimeline.push({
    id: Math.max(0, ...fixtureTimeline.map((row) => row.id)) + 1,
    threadId: 'th-lead',
    seq: nextSeq('th-lead'),
    kind: 'usage',
    payload: JSON.stringify({
      type: 'usage',
      used_tokens: usage.usedTokens ?? 0,
      context_size: usage.contextSize ?? 0,
      cost_usd: usage.costUsd,
    }),
    createdAt: new Date().toISOString().slice(0, 19).replace('T', ' '),
  })
}

/** Shapes the fixtures to the scenario, before the app first reads them. */
export function seedFixtures(scenario: HarnessScenario) {
  const persisted = fixtureSnapshot.persisted
  if (scenario.longThread && scenario.longThread > 0)
    seedLongThread(scenario.longThread)
  if (scenario.secondRepo) {
    persisted.repos.push({
      ...persisted.repos[0]!,
      id: 'api',
      name: 'api',
      path: '/tmp/cormux-fixture/api',
    })
  }
  if (scenario.timeline) {
    const others = fixtureTimeline.filter((row) => row.threadId !== 'th-lead')
    fixtureTimeline.length = 0
    fixtureTimeline.push(...others, ...scenario.timeline)
  }
  if (scenario.approvals) persisted.approvals = scenario.approvals
  if (scenario.reviewFindings) seedReviewFindings(scenario.reviewFindings)
  for (const [key, value] of Object.entries(scenario.settings ?? {})) {
    const row = persisted.settings.find((entry) => entry.key === key)
    if (row) row.value = value
    else persisted.settings.push({ key, value })
  }
  if (scenario.runCommand) persisted.repos[0]!.runCommand = scenario.runCommand
  if (scenario.leadUsage) seedLeadUsage(scenario.leadUsage)
  if (scenario.entities) persisted.entities = scenario.entities
  if (scenario.pullRequests) {
    persisted.pullRequests = scenario.pullRequests
    fixtureSnapshot.githubAuthConfigured = true
  }
  if (scenario.clickup) {
    fixtureSnapshot.clickupConfigured = true
    persisted.settings.push(
      { key: 'clickupWorkspaceId', value: 'team-1' },
      { key: 'clickupSpaceId', value: 'space-1' },
      { key: 'clickupFolderId', value: 'folder-sprints' },
    )
  }
}
