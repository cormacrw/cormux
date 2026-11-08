import type { TranscriptEvent } from '$lib/ipc/bindings'
import { appendEvent, findThread, findWorkspace, type HarnessDb } from './db'
import { LEAD_THREAD_ID, REPO_ID, WORKSPACE_ID } from './fixtures'
import type { HarnessScenario } from './scenario'

const SEEDED_AT = '2026-09-28 12:00:00'

function seedLongThread(db: HarnessDb, turns: number) {
  const push = (event: TranscriptEvent) =>
    appendEvent(db, LEAD_THREAD_ID, event, SEEDED_AT)
  for (let turn = 1; turn <= turns; turn += 1) {
    push({ type: 'messageChunk', role: 'user', text: `Question ${turn}` })
    push({ type: 'messageChunk', role: 'agent', text: `Answer ${turn}` })
    push({ type: 'turnEnd', stop_reason: 'end_turn', error: null })
  }
  // Switching away from a branch and back writes two notes with the same title.
  const switches = ['feat/test', 'feat/colors', 'feat/test']
  switches.forEach((branch, index) => {
    const previous = switches[index - 1] ?? 'main'
    push({
      type: 'step',
      id: `step-switch-${index}`,
      title: `Switched to \`${branch}\``,
      detail: 'Checked out in this worktree',
      step: {
        kind: 'branchSwitched',
        branch,
        change: { kind: 'checkedOut', previous },
      },
    })
  })
}

function seedReviewFindings(
  db: HarnessDb,
  findings: NonNullable<HarnessScenario['reviewFindings']>,
) {
  const persisted = db.snapshot.persisted
  const { row, record } = findWorkspace(db, WORKSPACE_ID)
  row!.kind = 'review'
  row!.prNumber = 2
  record!.kind = 'review'
  Object.assign(findThread(db, LEAD_THREAD_ID)!, {
    title: 'Reviewer',
    role: 'reviewer',
  })
  persisted.findings = findings
  persisted.reviews = [
    { workspaceId: WORKSPACE_ID, status: 'ready', submitted: false },
  ]
  // The core records the card in the thread that ran the review.
  appendEvent(
    db,
    LEAD_THREAD_ID,
    {
      type: 'step',
      id: 'step-findings-ready',
      title: `Review found ${findings.length} findings`,
      detail: null,
      step: { kind: 'reviewFindingsReady', count: findings.length },
    },
    SEEDED_AT,
  )
}

function seedLeadUsage(
  db: HarnessDb,
  usage: NonNullable<HarnessScenario['leadUsage']>,
) {
  Object.assign(findThread(db, LEAD_THREAD_ID)!, usage)
  // As a real turn does, log the usage it reported, stamped now.
  appendEvent(db, LEAD_THREAD_ID, {
    type: 'usage',
    used_tokens: usage.usedTokens ?? 0,
    context_size: usage.contextSize ?? 0,
    cost_usd: usage.costUsd,
  })
}

/** Shapes a session's fresh fixtures to the scenario, before the app first reads them. */
export function seedFixtures(db: HarnessDb, scenario: HarnessScenario) {
  const { snapshot } = db
  const persisted = snapshot.persisted
  const repo = persisted.repos.find((row) => row.id === REPO_ID)!
  if (scenario.longThread && scenario.longThread > 0)
    seedLongThread(db, scenario.longThread)
  if (scenario.secondRepo) {
    persisted.repos.push({
      ...repo,
      id: 'api',
      name: 'api',
      path: '/tmp/cormux-fixture/api',
    })
  }
  if (scenario.timeline) {
    db.timeline = [
      ...db.timeline.filter((row) => row.threadId !== LEAD_THREAD_ID),
      ...scenario.timeline,
    ]
  }
  if (scenario.approvals) persisted.approvals = scenario.approvals
  if (scenario.reviewFindings) seedReviewFindings(db, scenario.reviewFindings)
  Object.assign(snapshot.settings, scenario.settings)
  if (scenario.runCommand) repo.runCommand = scenario.runCommand
  if (scenario.leadUsage) seedLeadUsage(db, scenario.leadUsage)
  if (scenario.entities) persisted.entities = scenario.entities
  if (scenario.pullRequests) {
    persisted.pullRequests = scenario.pullRequests
    snapshot.githubAuthConfigured = true
  }
  if (scenario.clickup) {
    snapshot.clickupConfigured = true
    Object.assign(snapshot.settings, {
      clickupWorkspaceId: 'team-1',
      clickupSpaceId: 'space-1',
      clickupFolderId: 'folder-sprints',
    })
  }
}
